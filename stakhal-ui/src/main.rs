slint::slint! {
    export component AppWindow inherits Window {
        width: 400px;
        height: 300px;
        title: "Hello StakHAL";
        Text {
            text: "Hello StakHAL";
            font-size: 24px;
            horizontal-alignment: center;
            vertical-alignment: center;
        }
    }
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;
    ui.run()
}
