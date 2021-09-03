fn main() {
    let mut config = naive_logger::Config::default();
    config.level.set(log::LevelFilter::Debug);
    naive_logger::init(&config);
    log::debug!("too young");
    log::info!("too simple");
    log::warn!("sometimes naive");
    log::error!("I'm angry!");
    naive_logger::shutdown();
}
