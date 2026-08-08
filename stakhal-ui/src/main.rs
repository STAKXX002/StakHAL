use gtk4::prelude::*;
use libadwaita as adw;


const APP_ID: &str = "org.stakhal.StakHAL";

fn main() {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &adw::Application) {
    let label = gtk4::Label::builder()
        .label("StakHAL — GTK4 rebuild in progress")
        .halign(gtk4::Align::Center)
        .valign(gtk4::Align::Center)
        .build();

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("StakHAL - STM32 Project Viewer")
        .default_width(1200)
        .default_height(800)
        .content(&label)
        .build();

    window.present();
}
