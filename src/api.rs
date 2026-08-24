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

        url.push_str("https://api.openweathermap.org/data/2.5/weather?lat=45.19&lon=5.72&units=metric&appid=").unwrap();
        url.push_str(API).unwrap();
        Self {
            wifi,
            url,
            tls_seed,
        }
    }

    pub async fn access_website(&self) -> WeatherResponse {
        let mut rx_buffer = [0; 4096 * 2];
        let mut tx_buffer = [0; 4096 * 2];

        let tls_config = TlsConfig::new(
            self.tls_seed,
            &mut rx_buffer,
            &mut tx_buffer,
            reqwless::client::TlsVerify::None,
        );

        let dns = DnsSocket::new(self.wifi);
        let tcp_state = TcpClientState::<1, 4096, 4096>::new();
        let tcp = TcpClient::new(self.wifi, &tcp_state);

        let mut client = HttpClient::new_with_tls(&tcp, &dns, tls_config);
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
