use crate::mk_static;
use chrono::{DateTime, Datelike, Timelike, Utc};
use defmt::info;
use dotenvy_macro::dotenv;
use embassy_net::Stack;
use embassy_net::dns::DnsSocket;
use embassy_net::tcp::client::{TcpClient, TcpClientState};
use heapless::String;
use reqwless::client::{HttpClient, TlsConfig, TlsVerify};
use reqwless::request::{Method, RequestBuilder};
use serde::{Deserialize, Serialize};
use serde_json_core;

#[derive(Serialize, Deserialize, Debug)]
pub struct WeatherData {
    pub temp: f64,
    pub feels_like: f64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WeatherResponse {
    pub main: WeatherData,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub dt: DateTime<Utc>,
}

const API: &str = dotenv!("API");

pub struct WeatherApi {
    wifi: Stack<'static>,
    url: String<120>,
    tls_seed: u64,
}

impl WeatherApi {
    pub fn new(wifi: Stack<'static>, tls_seed: u64) -> Self {
        let mut url = String::new();

        url.push_str("https://api.openweathermap.org/data/2.5/weather?lat=44.34&lon=10.99&units=metric&appid=").unwrap();
        url.push_str(API).unwrap();
        Self {
            wifi,
            url,
            tls_seed,
        }
    }

    pub async fn access_website(&self) -> WeatherResponse {
        let tcp_client = TcpClient::new(
            self.wifi,
            mk_static!(
            TcpClientState<1, 16384, 4096>, // 16KB RX, 4KB TX
                TcpClientState::<1, 16384, 4096>::new()
                        ),
        );
        let dns_client = DnsSocket::new(self.wifi);

        let tls_config = TlsConfig::new(
            self.tls_seed,
            mk_static!([u8; 16384], [0; 16384]),
            mk_static!([u8; 4096], [0; 4096]),
            TlsVerify::None,
        );

        let mut client = HttpClient::new_with_tls(&tcp_client, &dns_client, tls_config);
        info!("Making HTTPS request");

        let mut rx_buf = [0u8; 4096];

        let request_result = client.request(Method::GET, self.url.as_str()).await;

        let request = match request_result {
            Ok(req) => req,
            Err(e) => {
                // THIS WILL TELL US EXACTLY WHY IT IS FAILING!
                info!(
                    "NETWORK ERROR: Failed to build request: {:?}",
                    defmt::Debug2Format(&e)
                );

                // 2. Safely trap the board in an infinite loop instead of returning!
                loop {
                    embassy_time::Timer::after(embassy_time::Duration::from_secs(1)).await;
                }
            }
        };
        let mut request = request.headers(&[("Connection", "close")]);

        let response = request.send(&mut rx_buf).await.unwrap();
        match response.body().read_to_end().await {
            Ok(res) => {
                let (data, _): (WeatherResponse, _) = serde_json_core::de::from_slice(res).unwrap();
                data
            }
            Err(err) => {
                info!("Body error");
                panic!()
            }
        }
    }
}
