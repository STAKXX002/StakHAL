use pyo3::exceptions::{PyIOError, PyValueError};
use pyo3::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

// --- Struct Wrappers ---

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyPinConfig {
    #[pyo3(get)]
    pub pin: String,
    #[pyo3(get)]
    pub signal: String,
    #[pyo3(get)]
    pub label: Option<String>,
}

#[pymethods]
impl PyPinConfig {
    fn __repr__(&self) -> String {
        format!(
            "PinConfig(pin='{}', signal='{}', label={:?})",
            self.pin, self.signal, self.label
        )
    }
}

impl From<&stakhal_core::ioc::PinConfig> for PyPinConfig {
    fn from(c: &stakhal_core::ioc::PinConfig) -> Self {
        PyPinConfig {
            pin: c.pin.clone(),
            signal: c.signal.clone(),
            label: c.label.clone(),
        }
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyPeripheralConfig {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub mode: Option<String>,
    #[pyo3(get)]
    pub parameters: HashMap<String, String>,
}

#[pymethods]
impl PyPeripheralConfig {
    fn __repr__(&self) -> String {
        format!(
            "PeripheralConfig(name='{}', mode={:?})",
            self.name, self.mode
        )
    }
}

impl From<&stakhal_core::ioc::PeripheralConfig> for PyPeripheralConfig {
    fn from(p: &stakhal_core::ioc::PeripheralConfig) -> Self {
        PyPeripheralConfig {
            name: p.name.clone(),
            mode: p.mode.clone(),
            parameters: p.parameters.clone(),
        }
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyIocProject {
    #[pyo3(get)]
    pub mcu_family: String,
    #[pyo3(get)]
    pub mcu_name: String,
    #[pyo3(get)]
    pub pins: Vec<PyPinConfig>,
    #[pyo3(get)]
    pub peripherals: Vec<PyPeripheralConfig>,
    #[pyo3(get)]
    pub raw: HashMap<String, String>,
}

#[pymethods]
impl PyIocProject {
    fn __repr__(&self) -> String {
        format!(
            "IocProject(mcu_family='{}', mcu_name='{}', pins={}, peripherals={})",
            self.mcu_family,
            self.mcu_name,
            self.pins.len(),
            self.peripherals.len()
        )
    }
}

impl From<stakhal_core::ioc::IocProject> for PyIocProject {
    fn from(i: stakhal_core::ioc::IocProject) -> Self {
        PyIocProject {
            mcu_family: i.mcu_family,
            mcu_name: i.mcu_name,
            pins: i.pins.iter().map(PyPinConfig::from).collect(),
            peripherals: i.peripherals.iter().map(PyPeripheralConfig::from).collect(),
            raw: i.raw,
        }
    }
}

impl From<&PyIocProject> for stakhal_core::ioc::IocProject {
    fn from(p: &PyIocProject) -> Self {
        stakhal_core::ioc::IocProject {
            mcu_family: p.mcu_family.clone(),
            mcu_name: p.mcu_name.clone(),
            pins: p
                .pins
                .iter()
                .map(|pin| stakhal_core::ioc::PinConfig {
                    pin: pin.pin.clone(),
                    signal: pin.signal.clone(),
                    label: pin.label.clone(),
                })
                .collect(),
            peripherals: p
                .peripherals
                .iter()
                .map(|periph| stakhal_core::ioc::PeripheralConfig {
                    name: periph.name.clone(),
                    mode: periph.mode.clone(),
                    parameters: periph.parameters.clone(),
                })
                .collect(),
            raw: p.raw.clone(),
        }
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyUserRegion {
    #[pyo3(get)]
    pub tag: String,
    #[pyo3(get)]
    pub file: String,
    #[pyo3(get)]
    pub byte_range: (usize, usize),
    #[pyo3(get)]
    pub line_range: (usize, usize),
    #[pyo3(get)]
    pub begin_marker_range: (usize, usize),
    #[pyo3(get)]
    pub end_marker_range: (usize, usize),
}

#[pymethods]
impl PyUserRegion {
    fn __repr__(&self) -> String {
        format!(
            "UserRegion(tag='{}', file='{}', byte_range={:?}, line_range={:?})",
            self.tag, self.file, self.byte_range, self.line_range
        )
    }
}

impl From<&stakhal_core::source::UserRegion> for PyUserRegion {
    fn from(r: &stakhal_core::source::UserRegion) -> Self {
        PyUserRegion {
            tag: r.tag.clone(),
            file: r.file.to_string_lossy().to_string(),
            byte_range: r.byte_range,
            line_range: r.line_range,
            begin_marker_range: r.begin_marker_range,
            end_marker_range: r.end_marker_range,
        }
    }
}

impl From<&PyUserRegion> for stakhal_core::source::UserRegion {
    fn from(r: &PyUserRegion) -> Self {
        stakhal_core::source::UserRegion {
            tag: r.tag.clone(),
            file: PathBuf::from(&r.file),
            byte_range: r.byte_range,
            line_range: r.line_range,
            begin_marker_range: r.begin_marker_range,
            end_marker_range: r.end_marker_range,
        }
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyPvDeclaration {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub type_str: String,
    #[pyo3(get)]
    pub initial_value: Option<String>,
    #[pyo3(get)]
    pub is_pointer: bool,
    #[pyo3(get)]
    pub is_array: bool,
    #[pyo3(get)]
    pub array_dims: Option<String>,
    #[pyo3(get)]
    pub raw_text: String,
    #[pyo3(get)]
    pub byte_range: (usize, usize),
    #[pyo3(get)]
    pub line: usize,
}

#[pymethods]
impl PyPvDeclaration {
    fn __repr__(&self) -> String {
        format!(
            "PvDeclaration(name='{}', type_str='{}', initial_value={:?}, line={})",
            self.name, self.type_str, self.initial_value, self.line
        )
    }
}

impl From<&stakhal_core::source::PvDeclaration> for PyPvDeclaration {
    fn from(d: &stakhal_core::source::PvDeclaration) -> Self {
        PyPvDeclaration {
            name: d.name.clone(),
            type_str: d.type_str.clone(),
            initial_value: d.initial_value.clone(),
            is_pointer: d.is_pointer,
            is_array: d.is_array,
            array_dims: d.array_dims.clone(),
            raw_text: d.raw_text.clone(),
            byte_range: d.byte_range,
            line: d.line,
        }
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyGraphEdge {
    #[pyo3(get)]
    pub from_node: String,
    #[pyo3(get)]
    pub to_node: String,
    #[pyo3(get)]
    pub edge_type: String,
    #[pyo3(get)]
    pub generated: bool,
}

#[pymethods]
impl PyGraphEdge {
    fn __repr__(&self) -> String {
        format!(
            "GraphEdge(from='{}', to='{}', edge_type='{}')",
            self.from_node, self.to_node, self.edge_type
        )
    }
}

impl From<&stakhal_core::graph::GraphEdge> for PyGraphEdge {
    fn from(e: &stakhal_core::graph::GraphEdge) -> Self {
        let edge_type_str = match e.edge_type {
            stakhal_core::graph::EdgeType::Init => "Init",
            stakhal_core::graph::EdgeType::IrqEntry => "IrqEntry",
            stakhal_core::graph::EdgeType::HalDispatch => "HalDispatch",
            stakhal_core::graph::EdgeType::WeakOverride => "WeakOverride",
        };
        PyGraphEdge {
            from_node: e.from.clone(),
            to_node: e.to.clone(),
            edge_type: edge_type_str.to_string(),
            generated: e.generated,
        }
    }
}

impl From<&PyGraphEdge> for stakhal_core::graph::GraphEdge {
    fn from(e: &PyGraphEdge) -> Self {
        let edge_type = match e.edge_type.as_str() {
            "Init" => stakhal_core::graph::EdgeType::Init,
            "IrqEntry" => stakhal_core::graph::EdgeType::IrqEntry,
            "HalDispatch" => stakhal_core::graph::EdgeType::HalDispatch,
            _ => stakhal_core::graph::EdgeType::WeakOverride,
        };
        stakhal_core::graph::GraphEdge {
            from: e.from_node.clone(),
            to: e.to_node.clone(),
            edge_type,
            generated: e.generated,
        }
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyChainHeaderLayout {
    #[pyo3(get)]
    pub handler_id: String,
    #[pyo3(get)]
    pub label: String,
    #[pyo3(get)]
    pub x: f64,
    #[pyo3(get)]
    pub y: f64,
    #[pyo3(get)]
    pub w: f64,
    #[pyo3(get)]
    pub h: f64,
    #[pyo3(get)]
    pub is_collapsed: bool,
}

#[pymethods]
impl PyChainHeaderLayout {
    fn __repr__(&self) -> String {
        format!(
            "ChainHeaderLayout(handler_id='{}', label='{}', pos=({:.1}, {:.1}))",
            self.handler_id, self.label, self.x, self.y
        )
    }
}

impl From<&stakhal_core::graph::ChainHeaderLayout> for PyChainHeaderLayout {
    fn from(h: &stakhal_core::graph::ChainHeaderLayout) -> Self {
        PyChainHeaderLayout {
            handler_id: h.handler_id.clone(),
            label: h.label.clone(),
            x: h.x,
            y: h.y,
            w: h.w,
            h: h.h,
            is_collapsed: h.is_collapsed,
        }
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyGraphLayout {
    #[pyo3(get)]
    pub positions: HashMap<String, (f64, f64)>,
    #[pyo3(get)]
    pub headers: Vec<PyChainHeaderLayout>,
    #[pyo3(get)]
    pub bounds: (i32, i32),
}

#[pymethods]
impl PyGraphLayout {
    fn __repr__(&self) -> String {
        format!(
            "GraphLayout(nodes={}, headers={}, bounds={:?})",
            self.positions.len(),
            self.headers.len(),
            self.bounds
        )
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyUsageSite {
    #[pyo3(get)]
    pub line: usize,
    #[pyo3(get)]
    pub byte_range: (usize, usize),
    #[pyo3(get)]
    pub context_snippet: String,
}

#[pymethods]
impl PyUsageSite {
    fn __repr__(&self) -> String {
        format!(
            "UsageSite(line={}, snippet='{}')",
            self.line, self.context_snippet
        )
    }
}

impl From<&stakhal_core::source::UsageSite> for PyUsageSite {
    fn from(u: &stakhal_core::source::UsageSite) -> Self {
        PyUsageSite {
            line: u.line,
            byte_range: u.byte_range,
            context_snippet: u.context_snippet.clone(),
        }
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyRenderedLine {
    #[pyo3(get)]
    pub line_number: usize,
    #[pyo3(get)]
    pub text: String,
    #[pyo3(get)]
    pub tier: String,
}

#[pymethods]
impl PyRenderedLine {
    fn __repr__(&self) -> String {
        format!(
            "RenderedLine(line={}, tier='{}', text='{}')",
            self.line_number, self.tier, self.text
        )
    }
}

impl From<&stakhal_core::source::RenderedLine> for PyRenderedLine {
    fn from(l: &stakhal_core::source::RenderedLine) -> Self {
        let tier_str = match l.tier {
            stakhal_core::source::LineTier::Generated => "Generated",
            stakhal_core::source::LineTier::Normal => "Normal",
            stakhal_core::source::LineTier::Declaration => "Declaration",
            stakhal_core::source::LineTier::Usage => "Usage",
        };
        PyRenderedLine {
            line_number: l.line_number,
            text: l.text.clone(),
            tier: tier_str.to_string(),
        }
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyUserFunction {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub byte_range: (usize, usize),
    #[pyo3(get)]
    pub line: usize,
    #[pyo3(get)]
    pub is_hal_callback: bool,
}

#[pymethods]
impl PyUserFunction {
    fn __repr__(&self) -> String {
        format!(
            "UserFunction(name='{}', line={}, is_hal_callback={})",
            self.name, self.line, self.is_hal_callback
        )
    }
}

impl From<&stakhal_core::graph::user_call_graph::UserFunction> for PyUserFunction {
    fn from(f: &stakhal_core::graph::user_call_graph::UserFunction) -> Self {
        PyUserFunction {
            name: f.name.clone(),
            byte_range: f.byte_range,
            line: f.line,
            is_hal_callback: f.is_hal_callback,
        }
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyUserCallEdge {
    #[pyo3(get)]
    pub from_node: String,
    #[pyo3(get)]
    pub to_node: String,
}

#[pymethods]
impl PyUserCallEdge {
    fn __repr__(&self) -> String {
        format!("UserCallEdge(from='{}', to='{}')", self.from_node, self.to_node)
    }
}

impl From<&stakhal_core::graph::user_call_graph::UserCallEdge> for PyUserCallEdge {
    fn from(e: &stakhal_core::graph::user_call_graph::UserCallEdge) -> Self {
        PyUserCallEdge {
            from_node: e.from.clone(),
            to_node: e.to.clone(),
        }
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyUserCallGraph {
    #[pyo3(get)]
    pub functions: Vec<PyUserFunction>,
    #[pyo3(get)]
    pub edges: Vec<PyUserCallEdge>,
}

#[pymethods]
impl PyUserCallGraph {
    fn __repr__(&self) -> String {
        format!(
            "UserCallGraph(functions={}, edges={})",
            self.functions.len(),
            self.edges.len()
        )
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyProjectMeta {
    #[pyo3(get)]
    pub name: String,
    #[pyo3(get)]
    pub mcu_family: String,
    #[pyo3(get)]
    pub mcu_name: String,
    #[pyo3(get)]
    pub ioc_path: String,
    #[pyo3(get)]
    pub main_c_path: String,
}

#[pymethods]
impl PyProjectMeta {
    fn __repr__(&self) -> String {
        format!(
            "ProjectMeta(name='{}', mcu='{}')",
            self.name, self.mcu_name
        )
    }
}

impl From<&stakhal_core::ir::schema::ProjectMeta> for PyProjectMeta {
    fn from(m: &stakhal_core::ir::schema::ProjectMeta) -> Self {
        PyProjectMeta {
            name: m.name.clone(),
            mcu_family: m.mcu_family.clone(),
            mcu_name: m.mcu_name.clone(),
            ioc_path: m.ioc_path.to_string_lossy().to_string(),
            main_c_path: m.main_c_path.to_string_lossy().to_string(),
        }
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyProject {
    #[pyo3(get)]
    pub meta: PyProjectMeta,
    #[pyo3(get)]
    pub pins: Vec<PyPinConfig>,
    #[pyo3(get)]
    pub peripherals: Vec<PyPeripheralConfig>,
    #[pyo3(get)]
    pub user_regions: Vec<PyUserRegion>,
    #[pyo3(get)]
    pub loop_body: Option<PyUserRegion>,
    #[pyo3(get)]
    pub call_graph_edges: Vec<PyGraphEdge>,
    #[pyo3(get)]
    pub pv_declarations: Vec<PyPvDeclaration>,
}

#[pymethods]
impl PyProject {
    fn __repr__(&self) -> String {
        format!(
            "Project(name='{}', pins={}, periphs={}, regions={}, pv_decls={}, graph_edges={})",
            self.meta.name,
            self.pins.len(),
            self.peripherals.len(),
            self.user_regions.len(),
            self.pv_declarations.len(),
            self.call_graph_edges.len()
        )
    }
}

impl From<stakhal_core::ir::schema::Project> for PyProject {
    fn from(p: stakhal_core::ir::schema::Project) -> Self {
        PyProject {
            meta: PyProjectMeta::from(&p.meta),
            pins: p.pins.iter().map(PyPinConfig::from).collect(),
            peripherals: p.peripherals.iter().map(PyPeripheralConfig::from).collect(),
            user_regions: p.user_regions.iter().map(PyUserRegion::from).collect(),
            loop_body: p.loop_body.as_ref().map(PyUserRegion::from),
            call_graph_edges: p.call_graph_edges.iter().map(PyGraphEdge::from).collect(),
            pv_declarations: p.pv_declarations.iter().map(PyPvDeclaration::from).collect(),
        }
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyPinLocation {
    #[pyo3(get)]
    pub mcu_pin: String,
    #[pyo3(get)]
    pub morpho: Option<(String, u8)>,
    #[pyo3(get)]
    pub arduino: Option<(String, u8, String)>,
}

#[pymethods]
impl PyPinLocation {
    fn __repr__(&self) -> String {
        format!(
            "PinLocation(pin='{}', morpho={:?}, arduino={:?})",
            self.mcu_pin, self.morpho, self.arduino
        )
    }
}

impl From<&stakhal_core::nucleo_pinout::PinLocation> for PyPinLocation {
    fn from(l: &stakhal_core::nucleo_pinout::PinLocation) -> Self {
        PyPinLocation {
            mcu_pin: l.mcu_pin.to_string(),
            morpho: l.morpho.map(|(c, p)| (c.to_string(), p)),
            arduino: l.arduino.map(|(c, p, lbl)| (c.to_string(), p, lbl.to_string())),
        }
    }
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct PyReservedPin {
    #[pyo3(get)]
    pub mcu_pin: String,
    #[pyo3(get)]
    pub reason: String,
    #[pyo3(get)]
    pub severity: String,
}

#[pymethods]
impl PyReservedPin {
    fn __repr__(&self) -> String {
        format!(
            "ReservedPin(pin='{}', severity='{}')",
            self.mcu_pin, self.severity
        )
    }
}

impl From<&stakhal_core::nucleo_pinout::ReservedPin> for PyReservedPin {
    fn from(r: &stakhal_core::nucleo_pinout::ReservedPin) -> Self {
        let sev = match r.severity {
            stakhal_core::nucleo_pinout::ReservedSeverity::Critical => "Critical",
            stakhal_core::nucleo_pinout::ReservedSeverity::Caution => "Caution",
        };
        PyReservedPin {
            mcu_pin: r.mcu_pin.to_string(),
            reason: r.reason.to_string(),
            severity: sev.to_string(),
        }
    }
}

// --- Top Level Functions ---

#[pyfunction]
pub fn discover_project_files(project_dir: &str) -> PyResult<(String, String)> {
    let (ioc, main_c) = stakhal_core::ioc::discovery::discover_project_files(Path::new(project_dir))
        .map_err(|e| PyIOError::new_err(e.to_string()))?;
    Ok((
        ioc.to_string_lossy().to_string(),
        main_c.to_string_lossy().to_string(),
    ))
}

#[pyfunction]
pub fn parse_ioc(path: &str) -> PyResult<PyIocProject> {
    let project = stakhal_core::ioc::parse_ioc(Path::new(path))
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(PyIocProject::from(project))
}

#[pyfunction]
pub fn parse_ioc_str(source: &str) -> PyResult<PyIocProject> {
    let project = stakhal_core::ioc::parse_ioc_str(source)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(PyIocProject::from(project))
}

#[pyfunction]
pub fn scan_file(path: &str) -> PyResult<Vec<PyUserRegion>> {
    let regions = stakhal_core::source::scan_file(Path::new(path))
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(regions.iter().map(PyUserRegion::from).collect())
}

#[pyfunction]
pub fn scan_source(path: &str, source: &str) -> PyResult<Vec<PyUserRegion>> {
    let regions = stakhal_core::source::scan_source(Path::new(path), source)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(regions.iter().map(PyUserRegion::from).collect())
}

#[pyfunction]
pub fn find_loop_body_gap(regions: Vec<PyUserRegion>) -> Option<PyUserRegion> {
    let core_regions: Vec<stakhal_core::source::UserRegion> =
        regions.iter().map(stakhal_core::source::UserRegion::from).collect();
    stakhal_core::source::find_loop_body_gap(&core_regions)
        .as_ref()
        .map(PyUserRegion::from)
}

#[pyfunction]
pub fn is_byte_in_user_region(byte_offset: usize, regions: Vec<PyUserRegion>) -> bool {
    let core_regions: Vec<stakhal_core::source::UserRegion> =
        regions.iter().map(stakhal_core::source::UserRegion::from).collect();
    stakhal_core::source::is_byte_in_user_region(byte_offset, &core_regions)
}

#[pyfunction]
pub fn extract_pv_declarations(path: &str) -> PyResult<Vec<PyPvDeclaration>> {
    let decls = stakhal_core::source::extract_pv_declarations(Path::new(path))
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(decls.iter().map(PyPvDeclaration::from).collect())
}

#[pyfunction]
pub fn find_variable_usages(
    path: &str,
    variable_name: &str,
    decl_start: usize,
    decl_end: usize,
) -> PyResult<Vec<PyUsageSite>> {
    let usages = stakhal_core::source::find_variable_usages(
        Path::new(path),
        variable_name,
        (decl_start, decl_end),
    )
    .map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(usages.iter().map(PyUsageSite::from).collect())
}

#[pyfunction]
pub fn build_source_render_model(
    path: &str,
    regions: Vec<PyUserRegion>,
    decl_byte_range: (usize, usize),
    usage_byte_ranges: Vec<(usize, usize)>,
) -> PyResult<Vec<PyRenderedLine>> {
    let core_regions: Vec<stakhal_core::source::UserRegion> =
        regions.iter().map(stakhal_core::source::UserRegion::from).collect();
    let lines = stakhal_core::source::build_source_render_model(
        Path::new(path),
        &core_regions,
        decl_byte_range,
        &usage_byte_ranges,
    )
    .map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(lines.iter().map(PyRenderedLine::from).collect())
}

#[pyfunction]
pub fn write_region(path: &str, region: &PyUserRegion, new_content: &str) -> PyResult<()> {
    let core_region = stakhal_core::source::UserRegion::from(region);
    stakhal_core::source::write_region(Path::new(path), &core_region, new_content)
        .map_err(|e| PyIOError::new_err(e.to_string()))
}

#[pyfunction]
pub fn build_call_graph(ioc: &PyIocProject) -> Vec<PyGraphEdge> {
    let core_ioc = stakhal_core::ioc::IocProject::from(ioc);
    let edges = stakhal_core::graph::build_call_graph(&core_ioc);
    edges.iter().map(PyGraphEdge::from).collect()
}

#[pyfunction]
pub fn compute_graph_layout(
    edges: Vec<PyGraphEdge>,
    collapsed_chains: Vec<String>,
) -> PyGraphLayout {
    let core_edges: Vec<stakhal_core::graph::GraphEdge> =
        edges.iter().map(stakhal_core::graph::GraphEdge::from).collect();
    let collapsed_set: HashSet<String> = collapsed_chains.into_iter().collect();
    let (positions, headers) =
        stakhal_core::graph::compute_graph_layout(&core_edges, &collapsed_set);
    let bounds = stakhal_core::graph::compute_graph_bounds(&positions, &headers);
    let py_headers = headers.iter().map(PyChainHeaderLayout::from).collect();
    PyGraphLayout {
        positions,
        headers: py_headers,
        bounds,
    }
}

#[pyfunction]
pub fn build_user_call_graph(main_c_path: &str) -> PyResult<PyUserCallGraph> {
    let (fns, edges) = stakhal_core::graph::user_call_graph::build_user_call_graph(Path::new(main_c_path))
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(PyUserCallGraph {
        functions: fns.iter().map(PyUserFunction::from).collect(),
        edges: edges.iter().map(PyUserCallEdge::from).collect(),
    })
}

#[pyfunction]
pub fn load_project(ioc_path: &str, main_c_path: &str) -> PyResult<PyProject> {
    let project = stakhal_core::ir::schema::load_project(Path::new(ioc_path), Path::new(main_c_path))
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(PyProject::from(project))
}

#[pyfunction]
pub fn load_project_from_dir(project_dir: &str) -> PyResult<PyProject> {
    let (ioc_path, main_c_path) =
        stakhal_core::ioc::discovery::discover_project_files(Path::new(project_dir))
            .map_err(|e| PyIOError::new_err(e.to_string()))?;
    load_project(&ioc_path.to_string_lossy(), &main_c_path.to_string_lossy())
}

#[pyfunction]
pub fn lookup_pin(mcu_pin: &str) -> Option<PyPinLocation> {
    stakhal_core::nucleo_pinout::lookup_pin(mcu_pin).map(PyPinLocation::from)
}

#[pyfunction]
pub fn check_reserved(mcu_pin: &str) -> Option<PyReservedPin> {
    stakhal_core::nucleo_pinout::check_reserved(mcu_pin).map(PyReservedPin::from)
}

#[pymodule]
fn stakhal_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPinConfig>()?;
    m.add_class::<PyPeripheralConfig>()?;
    m.add_class::<PyIocProject>()?;
    m.add_class::<PyUserRegion>()?;
    m.add_class::<PyPvDeclaration>()?;
    m.add_class::<PyGraphEdge>()?;
    m.add_class::<PyChainHeaderLayout>()?;
    m.add_class::<PyGraphLayout>()?;
    m.add_class::<PyUsageSite>()?;
    m.add_class::<PyRenderedLine>()?;
    m.add_class::<PyUserFunction>()?;
    m.add_class::<PyUserCallEdge>()?;
    m.add_class::<PyUserCallGraph>()?;
    m.add_class::<PyProjectMeta>()?;
    m.add_class::<PyProject>()?;
    m.add_class::<PyPinLocation>()?;
    m.add_class::<PyReservedPin>()?;

    m.add_function(wrap_pyfunction!(discover_project_files, m)?)?;
    m.add_function(wrap_pyfunction!(parse_ioc, m)?)?;
    m.add_function(wrap_pyfunction!(parse_ioc_str, m)?)?;
    m.add_function(wrap_pyfunction!(scan_file, m)?)?;
    m.add_function(wrap_pyfunction!(scan_source, m)?)?;
    m.add_function(wrap_pyfunction!(find_loop_body_gap, m)?)?;
    m.add_function(wrap_pyfunction!(is_byte_in_user_region, m)?)?;
    m.add_function(wrap_pyfunction!(extract_pv_declarations, m)?)?;
    m.add_function(wrap_pyfunction!(find_variable_usages, m)?)?;
    m.add_function(wrap_pyfunction!(build_source_render_model, m)?)?;
    m.add_function(wrap_pyfunction!(write_region, m)?)?;
    m.add_function(wrap_pyfunction!(build_call_graph, m)?)?;
    m.add_function(wrap_pyfunction!(compute_graph_layout, m)?)?;
    m.add_function(wrap_pyfunction!(build_user_call_graph, m)?)?;
    m.add_function(wrap_pyfunction!(load_project, m)?)?;
    m.add_function(wrap_pyfunction!(load_project_from_dir, m)?)?;
    m.add_function(wrap_pyfunction!(lookup_pin, m)?)?;
    m.add_function(wrap_pyfunction!(check_reserved, m)?)?;

    Ok(())
}
