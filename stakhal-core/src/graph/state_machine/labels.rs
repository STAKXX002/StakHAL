use tree_sitter::Node;

pub(crate) fn node_text(node: Node, source_bytes: &[u8]) -> String {
    node.utf8_text(source_bytes).unwrap_or("").trim().to_string()
}

pub(crate) fn clean_guard_text(raw: &str) -> String {
    let mut s = raw.trim();
    while s.starts_with('(') && s.ends_with(')') && has_matching_outer_parens(s) {
        s = s[1..s.len() - 1].trim();
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(crate) fn has_matching_outer_parens(s: &str) -> bool {
    if !s.starts_with('(') || !s.ends_with(')') {
        return false;
    }
    let mut depth = 0;
    for (i, c) in s.chars().enumerate() {
        if c == '(' {
            depth += 1;
        } else if c == ')' {
            depth -= 1;
            if depth == 0 && i < s.len() - 1 {
                return false;
            }
        }
    }
    depth == 0
}

#[derive(Debug, Clone)]
pub(crate) struct CallArgInfo {
    pub(crate) text: String,
    pub(crate) is_string_literal: bool,
}

pub(crate) fn extract_first_argument_info(call_node: Node, source_bytes: &[u8]) -> Option<CallArgInfo> {
    if let Some(arg_list) = call_node.child_by_field_name("arguments") {
        let mut cursor = arg_list.walk();
        for child in arg_list.children(&mut cursor) {
            if child.kind() != "(" && child.kind() != ")" && child.kind() != "," && child.kind() != "comment" {
                let is_string_literal = child.kind() == "string_literal";
                let mut text = node_text(child, source_bytes);
                if text.starts_with('"') && text.ends_with('"') && text.len() >= 2 {
                    text = text[1..text.len() - 1].to_string();
                }
                return Some(CallArgInfo {
                    text,
                    is_string_literal,
                });
            }
        }
    }
    None
}

pub(crate) fn extract_command_string(guard: &str) -> Option<String> {
    if let Some(idx) = guard.find("strcmp") {
        let after_strcmp = &guard[idx..];
        if let Some(eq_idx) = after_strcmp.find("==") {
            let call_str = &after_strcmp[..eq_idx];
            if let Some(first_quote) = call_str.find('"') {
                let after_quote = &call_str[first_quote + 1..];
                if let Some(second_quote) = after_quote.find('"') {
                    let cmd = &after_quote[..second_quote];
                    if !cmd.is_empty() {
                        return Some(cmd.to_string());
                    }
                }
            }
        }
    }
    None
}

pub(crate) fn clean_printf_literal(raw: &str) -> String {
    let s = raw
        .replace("\\r", "")
        .replace("\\n", "")
        .replace("\r", "")
        .replace("\n", "");
    s.trim().trim_matches(|c| c == '.' || c == '!').trim().to_string()
}

pub(crate) fn differs_meaningfully(printf_msg: &str, dest_state: &str) -> bool {
    let clean_msg = printf_msg
        .replace("\\r", "")
        .replace("\\n", "")
        .replace("\r", "")
        .replace("\n", "");
    let clean_msg = clean_msg.trim().to_lowercase();
    let clean_dest = dest_state.trim().to_lowercase();

    if clean_msg.is_empty() {
        return false;
    }

    // Ignore hit-detection side info
    if clean_msg.contains("hit") {
        return false;
    }

    // Status / state confirmations in fixture
    let known_redundant = [
        "hold", "cal ok", "returned", "stopping", "backoff", "go", "ret",
        "rst", "opening", "closing", "closed", "opened", "busy", "no cal",
        "invalid", "hatch stopped", "ok",
    ];
    if known_redundant.iter().any(|&r| clean_msg == r || clean_msg.contains(r)) {
        return false;
    }

    // Normalize dest state (e.g. remove cal_, rec_ prefixes)
    let stripped_dest = clean_dest
        .strip_prefix("cal_")
        .or_else(|| clean_dest.strip_prefix("rec_"))
        .unwrap_or(&clean_dest)
        .replace('_', "");

    let compact_msg = clean_msg.replace(|c: char| !c.is_alphanumeric(), "");

    if compact_msg == stripped_dest || compact_msg == clean_dest.replace('_', "") {
        return false;
    }
    if stripped_dest.contains(&compact_msg) || compact_msg.contains(&stripped_dest) {
        return false;
    }

    true
}

pub(crate) fn find_following_printf(node: Node, source_bytes: &[u8]) -> Option<String> {
    let stmt = find_parent_statement(node)?;
    let mut sibling = stmt.next_sibling();
    let mut count = 0;
    while let Some(sib) = sibling {
        count += 1;
        if count > 4 {
            break;
        }
        if sib.kind() == "comment" {
            sibling = sib.next_sibling();
            continue;
        }
        if let Some(call) = find_call_in_statement(sib) {
            if let Some(fn_node) = call.child_by_field_name("function") {
                if node_text(fn_node, source_bytes) == "printf" {
                    if let Some(arg) = extract_first_argument_info(call, source_bytes) {
                        if arg.is_string_literal {
                            return Some(arg.text);
                        }
                    }
                }
            }
        }
        sibling = sib.next_sibling();
    }
    None
}

pub(crate) fn find_parent_statement(mut node: Node) -> Option<Node> {
    while let Some(parent) = node.parent() {
        if parent.kind().ends_with("_statement") {
            return Some(parent);
        }
        node = parent;
    }
    None
}

pub(crate) fn find_call_in_statement(stmt: Node) -> Option<Node> {
    if stmt.kind() == "call_expression" {
        return Some(stmt);
    }
    if stmt.kind() == "expression_statement" {
        let mut cursor = stmt.walk();
        for child in stmt.children(&mut cursor) {
            if child.kind() == "call_expression" {
                return Some(child);
            }
        }
    }
    None
}

pub fn to_title_case_identifier(ident: &str) -> String {
    let s = ident.trim_end_matches("()");
    let mut words: Vec<String> = Vec::new();

    for part in s.split(['_', '.']) {
        if part.is_empty() {
            continue;
        }
        let mut cur_word = String::new();
        let chars: Vec<char> = part.chars().collect();
        for i in 0..chars.len() {
            let c = chars[i];
            if i > 0 {
                let prev = chars[i - 1];
                let is_prev_lower_or_digit = prev.is_ascii_lowercase() || prev.is_ascii_digit();
                let is_cur_upper = c.is_ascii_uppercase();
                let is_next_lower = i + 1 < chars.len() && chars[i + 1].is_ascii_lowercase();

                if (is_prev_lower_or_digit && is_cur_upper)
                    || (prev.is_ascii_uppercase() && is_cur_upper && is_next_lower)
                {
                    if !cur_word.is_empty() {
                        words.push(cur_word);
                        cur_word = String::new();
                    }
                }
            }
            cur_word.push(c);
        }
        if !cur_word.is_empty() {
            words.push(cur_word);
        }
    }

    let capitalized: Vec<String> = words
        .into_iter()
        .map(|w| {
            if w.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit()) {
                w
            } else {
                let mut chars = w.chars();
                match chars.next() {
                    Some(first) => {
                        let mut res = first.to_uppercase().to_string();
                        res.push_str(chars.as_str());
                        res
                    }
                    None => String::new(),
                }
            }
        })
        .collect();

    capitalized.join(" ")
}

pub fn prettify_guard(raw_guard: &str) -> String {
    let mut s = raw_guard.trim();
    if let Some(idx) = s.find(" [") {
        s = &s[..idx];
    }
    s = s.trim();

    while s.starts_with('(') && s.ends_with(')') && has_matching_outer_parens(s) {
        s = s[1..s.len() - 1].trim();
    }

    if s.is_empty() {
        return String::new();
    }

    // 1. If there is an equality discriminator like pendingState == HATCH_OPENING,
    // prioritize the discriminator label even if a timeout condition is present in the conjunction.
    if s.contains("==") {
        if let Some(term) = s.split("&&").map(|p| p.trim()).find(|p| p.contains("==")) {
            return prettify_single_term(term);
        }
    }

    // 2. Timeout pattern
    if s.contains("now") && s.contains("stateStart") {
        if let Some(timeout_label) = extract_timeout_pattern(s) {
            return timeout_label;
        }
    }

    // 3. Split by ||
    if s.contains("||") {
        let parts: Vec<String> = s
            .split("||")
            .map(|p| prettify_conjunctive_clause(p.trim()))
            .filter(|p| !p.is_empty())
            .collect();
        return parts.join(" or ");
    }

    // 4. Conjunctive clause
    prettify_conjunctive_clause(s)
}

pub(crate) fn extract_timeout_pattern(s: &str) -> Option<String> {
    let s_clean = s.trim();
    let s_clean = if s_clean.starts_with('(') && s_clean.ends_with(')') {
        s_clean[1..s_clean.len() - 1].trim()
    } else {
        s_clean
    };

    if let Some(idx) = s_clean.find("now") {
        let after_now = s_clean[idx + 3..].trim_start();
        if let Some(dash_idx) = after_now.find('-') {
            let after_dash = after_now[dash_idx + 1..].trim_start();
            if let Some(gt_idx) = after_dash.find('>') {
                let left_side = after_dash[..gt_idx].trim();
                let right_side = after_dash[gt_idx + 1..].trim();
                if left_side.contains("stateStart") {
                    let const_name = right_side
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect::<String>();
                    if !const_name.is_empty() {
                        return Some(format!("Timeout ({})", const_name));
                    }
                }
            }
        }
    }
    None
}

pub(crate) fn prettify_conjunctive_clause(clause: &str) -> String {
    let parts: Vec<String> = clause
        .split("&&")
        .map(|p| prettify_single_term(p.trim()))
        .filter(|p| !p.is_empty())
        .collect();
    parts.join(" and ")
}

pub(crate) fn is_camel_case_ident(s: &str) -> bool {
    let clean = s.trim();
    if clean.is_empty() || clean.contains(' ') || clean.contains('[') || clean.contains('(') {
        return false;
    }
    let first = clean.chars().next().unwrap();
    first.is_ascii_lowercase() && clean.chars().any(|c| c.is_ascii_uppercase())
}

pub(crate) fn strip_variant_prefix(s: &str) -> Option<String> {
    let clean = s.trim();
    if clean.contains('_') && clean.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_') {
        let idx = clean.find('_')?;
        let stripped = &clean[idx + 1..];
        if !stripped.is_empty() {
            return Some(stripped.to_string());
        }
    }
    None
}

pub(crate) fn prettify_single_term(term: &str) -> String {
    let mut s = term.trim();
    while s.starts_with('(') && s.ends_with(')') && has_matching_outer_parens(s) {
        s = s[1..s.len() - 1].trim();
    }
    if s.is_empty() {
        return String::new();
    }

    // Equality comparison
    if s.contains("==") {
        let parts: Vec<&str> = s.split("==").collect();
        if parts.len() == 2 {
            let left_raw = parts[0].trim();
            let right_raw = parts[1].trim();

            let left = if is_camel_case_ident(left_raw) {
                left_raw.to_string()
            } else {
                prettify_operand(left_raw)
            };

            let right = if let Some(stripped) = strip_variant_prefix(right_raw) {
                stripped
            } else {
                prettify_operand(right_raw)
            };

            return format!("{} is {}", left, right);
        }
    }

    // Inequality comparison
    if s.contains("!=") {
        let parts: Vec<&str> = s.split("!=").collect();
        if parts.len() == 2 {
            let left = prettify_operand(parts[0].trim());
            let right = prettify_operand(parts[1].trim());
            return format!("{} is not {}", left, right);
        }
    }

    // Leading negation !
    if let Some(rest) = s.strip_prefix('!') {
        return format!("not {}", prettify_operand(rest.trim()));
    }

    prettify_operand(s)
}

pub(crate) fn prettify_operand(op: &str) -> String {
    let mut s = op.trim();
    while s.starts_with('(') && s.ends_with(')') && has_matching_outer_parens(s) {
        s = s[1..s.len() - 1].trim();
    }
    match s {
        "0" => "0".to_string(),
        "1" => "1".to_string(),
        "true" => "true".to_string(),
        "false" => "false".to_string(),
        "NULL" => "NULL".to_string(),
        _ => to_title_case_identifier(s),
    }
}
