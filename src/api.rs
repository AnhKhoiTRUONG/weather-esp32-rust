use chrono::{DateTime, Utc};
use defmt::info;
use dotenvy_macro::dotenv;
use embassy_net::Stack;
use embassy_net::dns::DnsSocket;
use embassy_net::tcp::client::{TcpClient, TcpClientState};
use heapless::{String, Vec};
use reqwless::client::{HttpClient, TlsConfig};
use reqwless::request::{Method, RequestBuilder};
use serde::{Deserialize, Serialize};
use serde_json_core;
use serde_repr::Deserialize_repr;

#[derive(Serialize, Deserialize, Debug)]
pub struct WeatherData {
    pub temp: f64,
    pub feels_like: f64,
}

#[derive(Deserialize, Debug)]
pub struct WeatherResponse {
    pub weather: Vec<Weather, 1>,
    pub main: WeatherData,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub dt: DateTime<Utc>,
}

#[derive(Deserialize, Debug)]
pub struct Weather {
    pub id: ConditionCode,
}

const API: &str = dotenv!("API");

pub struct WeatherApi {
    wifi: Stack<'static>,
    url: String<120>,
    tls_seed: u64,
}

#[derive(Debug, Deserialize_repr)]
#[repr(u16)]
pub enum ConditionCode {
    // Group 2xx: Thunderstorm
    ThunderstormWithLightRain = 200,
    ThunderstormWithRain = 201,
    ThunderstormWithHeavyRain = 202,
    LightThunderstorm = 210,
    Thunderstorm = 211,
    HeavyThunderstorm = 212,
    RaggedThunderstorm = 221,
    ThunderstormWithLightDrizzle = 230,
    ThunderstormWithDrizzle = 231,
    ThunderstormWithHeavyDrizzle = 232,

    // Group 3xx: Drizzle
    LightIntensityDrizzle = 300,
    Drizzle = 301,
    HeavyIntensityDrizzle = 302,
    LightIntensityDrizzleRain = 310,
    DrizzleRain = 311,
    HeavyIntensityDrizzleRain = 312,
    ShowerRainAndDrizzle = 313,
    HeavyShowerRainAndDrizzle = 314,
    ShowerDrizzle = 321,

    // Group 5xx: Rain
    LightRain = 500,
    ModerateRain = 501,
    HeavyIntensityRain = 502,
    VeryHeavyRain = 503,
    ExtremeRain = 504,
    FreezingRain = 511,
    LightIntensityShowerRain = 520,
    ShowerRain = 521,
    HeavyIntensityShowerRain = 522,
    RaggedShowerRain = 531,

    // Group 6xx: Snow
    LightSnow = 600,
    Snow = 601,
    HeavySnow = 602,
    Sleet = 611,
    LightShowerSleet = 612,
    ShowerSleet = 613,
    LightRainAndSnow = 615,
    RainAndSnow = 616,
    LightShowerSnow = 620,
    ShowerSnow = 621,
    HeavyShowerSnow = 622,

    // Group 7xx: Atmosphere
    Mist = 701,
    Smoke = 711,
    Haze = 721,
    SandDustWhirls = 731,
    Fog = 741,
    Sand = 751,
    Dust = 761,
    VolcanicAsh = 762,
    Squalls = 771,
    Tornado = 781,

    // Group 800: Clear
    ClearSky = 800,

    // Group 80x: Clouds
    FewClouds = 801,
    ScatteredClouds = 802,
    BrokenClouds = 803,
    OvercastClouds = 804,
}

impl ConditionCode {
    pub fn icon(&self) -> &'static str {
        match self {
            // Thunderstorm
            ConditionCode::ThunderstormWithLightRain
            | ConditionCode::ThunderstormWithRain
            | ConditionCode::ThunderstormWithHeavyRain
            | ConditionCode::LightThunderstorm
            | ConditionCode::Thunderstorm
            | ConditionCode::HeavyThunderstorm
            | ConditionCode::RaggedThunderstorm
            | ConditionCode::ThunderstormWithLightDrizzle
            | ConditionCode::ThunderstormWithDrizzle
            | ConditionCode::ThunderstormWithHeavyDrizzle => "storm.bmp",

            // Drizzle
            ConditionCode::LightIntensityDrizzle
            | ConditionCode::Drizzle
            | ConditionCode::HeavyIntensityDrizzle
            | ConditionCode::LightIntensityDrizzleRain
            | ConditionCode::DrizzleRain
            | ConditionCode::HeavyIntensityDrizzleRain
            | ConditionCode::ShowerRainAndDrizzle
            | ConditionCode::HeavyShowerRainAndDrizzle
            | ConditionCode::ShowerDrizzle => "rainy.bmp",

            // Rain
            ConditionCode::LightRain
            | ConditionCode::ModerateRain
            | ConditionCode::HeavyIntensityRain
            | ConditionCode::VeryHeavyRain
            | ConditionCode::ExtremeRain
            | ConditionCode::LightIntensityShowerRain
            | ConditionCode::ShowerRain
            | ConditionCode::HeavyIntensityShowerRain
            | ConditionCode::RaggedShowerRain => "rainy_heavy.bmp",
            ConditionCode::FreezingRain => "weather_mix.bmp",

            // Snow
            ConditionCode::LightSnow
            | ConditionCode::Snow
            | ConditionCode::HeavySnow
            | ConditionCode::Sleet
            | ConditionCode::LightShowerSleet
            | ConditionCode::ShowerSleet
            | ConditionCode::LightRainAndSnow
            | ConditionCode::RainAndSnow
            | ConditionCode::LightShowerSnow
            | ConditionCode::ShowerSnow
            | ConditionCode::HeavyShowerSnow => "snowing.bmp",

            // Atmosphere
            ConditionCode::Mist
            | ConditionCode::Smoke
            | ConditionCode::Haze
            | ConditionCode::SandDustWhirls
            | ConditionCode::Fog
            | ConditionCode::Sand
            | ConditionCode::Dust
            | ConditionCode::VolcanicAsh
            | ConditionCode::Squalls => "foggy.bmp",
            ConditionCode::Tornado => "cyclone.bmp",

            // Clear
            ConditionCode::ClearSky => "sunny.bmp",

            // Clouds
            ConditionCode::FewClouds
            | ConditionCode::ScatteredClouds
            | ConditionCode::BrokenClouds
            | ConditionCode::OvercastClouds => "partly_cloudy_day.bmp",
        }
    }
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
