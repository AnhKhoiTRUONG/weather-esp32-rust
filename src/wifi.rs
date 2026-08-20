use crate::mk_static;
use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_net::{Config as NetConfig, DhcpConfig, Runner, Stack, StackResources};
use embassy_time::{Duration, Timer};
use esp_hal::rng::Rng;
use esp_radio::wifi::{
    Config, ControllerConfig, Interface, WifiController, scan::ScanConfig, sta::StationConfig,
};

// macro_rules! mk_static {
//     ($t:ty, $val:expr) => {{
//         static STATIC_CELL: static_cell::StaticCell<$t> = static_cell::StaticCell::new();
//         STATIC_CELL.uninit().write($val)
//     }};
// }

#[embassy_executor::task]
pub async fn connection(mut controller: WifiController<'static>) {
    info!("Starting Wifi connection ...");

    info!("Scanning for Wi-Fi networks...");
    let scan_config = ScanConfig::default();

    match controller.scan_async(&scan_config).await {
        Ok(networks) => {
            info!("Found {} networks:", networks.len());
            for ap in networks {
                // We print the SSID (name)
                info!("SSID: {}", ap.ssid.as_str());
            }
        }
        Err(e) => {
            info!("Scan failed: {:?}", e);
        }
    }

    info!("Scan complete! Now attempting to connect to your network...");
    loop {
        match controller.connect_async().await {
            Ok(_) => {
                info!("Wifi connected!");
                let _ = controller.wait_for_disconnect_async().await;
                info!("Wifi disconnected!");
            }
            Err(e) => info!("Failed to connect to Wi-Fi: {:?},{:?}", SSID, e),
        }

        Timer::after(Duration::from_secs(5)).await;
    }
}
#[embassy_executor::task]
async fn net_task(mut runner: Runner<'static, Interface<'static>>) {
    runner.run().await
}

const SSID: &str = env!("SSID");
const PASS: &str = env!("PASS");

#[allow(clippy::large_stack_frames)]
pub async fn start_wifi(
    peripherals_wifi: esp_hal::peripherals::WIFI<'static>,
    rng: Rng,
    spawner: &Spawner,
) -> Stack<'static> {
    let station_config = Config::Station(
        StationConfig::default()
            .with_ssid(SSID)
            .with_password(PASS.into()),
    );

    let (wifi_controller, interfaces) = esp_radio::wifi::new(
        peripherals_wifi,
        ControllerConfig::default().with_initial_config(station_config),
    )
    .expect("Failed to initialize Wi-Fi controller");

    let seed = (rng.random() as u64) << 32 | rng.random() as u64;

    let (stack, runner) = embassy_net::new(
        interfaces.station,
        NetConfig::dhcpv4(DhcpConfig::default()),
        mk_static!(StackResources<5>, StackResources::new()),
        seed,
    );

    spawner.spawn(connection(wifi_controller).unwrap());
    spawner.spawn(net_task(runner).unwrap());

    wait_for_connection(stack).await;
    stack
}

pub async fn wait_for_connection(stack: Stack<'_>) {
    info!("Waiting for DHCP IP address...");
    stack.wait_config_up().await;

    if let Some(config) = stack.config_v4() {
        info!("Successfully got IP: {}", config.address);
    }
    Timer::after(Duration::from_secs(3)).await;
}
