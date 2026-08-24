use embassy_time::{Delay, Duration, Timer};
use embedded_graphics::pixelcolor::BinaryColor;
use embedded_graphics::primitives::{Line, PrimitiveStyle};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::gpio::{Input, Output};
use esp_println as _;
use tinybmp::Bmp;

// SPI
use esp_hal::spi::master::Spi;

// epd
use epd_waveshare::epd2in13_v2::{Display2in13, Epd2in13};
use epd_waveshare::graphics::DisplayRotation;
use epd_waveshare::prelude::{Color, WaveshareDisplay};

// embedded graphics
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::mono_font::iso_8859_1::FONT_10X20;
use embedded_graphics::prelude::*;
// use embedded_graphics::primitives::{Circle, PrimitiveStyle, PrimitiveStyleBuilder};
use embedded_graphics::text::{Baseline, Text};

use embassy_net::Stack;

use crate::api::{ConditionCode, WeatherApi, WeatherData, WeatherResponse};
use crate::icon::ICONS;

use chrono::{DateTime, Datelike, Timelike, Utc};
use core::fmt::Write;
// use core::time::Duration;
use heapless::String;

use defmt::info;

type SpiDevice = ExclusiveDevice<Spi<'static, esp_hal::Blocking>, Output<'static>, Delay>;
type EPD = Epd2in13<SpiDevice, Input<'static>, Output<'static>, Output<'static>, Delay>;

pub struct DashBoard {
    display: Display2in13,
    wifi: Stack<'static>,
    epd: EPD,
    spi_dev: SpiDevice,
}

impl DashBoard {
    pub fn new(display: Display2in13, wifi: Stack<'static>, epd: EPD, spi_dev: SpiDevice) -> Self {
        Self {
            display,
            wifi,
            epd,
            spi_dev,
        }
    }

    pub async fn start(&mut self, tls_seed: u64) {
        self.display.set_rotation(DisplayRotation::Rotate90);

        let api = WeatherApi::new(self.wifi, tls_seed);

        loop {
            let data = WeatherApi::access_website(&api).await;

            self.refresh(data).await;

            Timer::after(Duration::from_secs(600)).await;
        }
    }

    pub async fn refresh(&mut self, data: WeatherResponse) {
        self.epd.wake_up(&mut self.spi_dev, &mut Delay).unwrap();

        Timer::after(Duration::from_secs(5)).await;

        self.epd.clear_frame(&mut self.spi_dev, &mut Delay).unwrap();
        self.display.clear(Color::White).unwrap();
        self.epd
            .update_and_display_frame(&mut self.spi_dev, self.display.buffer(), &mut Delay)
            .unwrap();
        Timer::after(Duration::from_secs(5)).await;

        self.draw_date(data.dt).await;
        self.draw_temp(data.main).await;
        self.draw_weather_icon(&data.weather[0].id).await;

        self.epd
            .update_and_display_frame(&mut self.spi_dev, self.display.buffer(), &mut Delay)
            .unwrap();
        Timer::after(Duration::from_secs(5)).await;

        self.epd.sleep(&mut self.spi_dev, &mut Delay).unwrap();
    }

    pub async fn draw_date(&mut self, dt: DateTime<Utc>) {
        info!("Draw date");
        info!(
            "Date: {:02}-{:02}-{:04}\nTime: {:02}:{:02}",
            dt.day(),
            dt.month(),
            dt.year(),
            dt.hour(),
            dt.minute(),
        );
        let mut text: String<50> = String::new();
        write!(
            &mut text,
            "{:02} {:02} {:04}",
            dt.day(),
            month_name(dt.month()),
            dt.year()
        )
        .unwrap();
        draw_text(&mut self.display, text.as_str(), 60, 0);
        Line::new(Point::new(0, 22), Point::new(260, 22))
            .into_styled(PrimitiveStyle::with_stroke(Color::Black, 5))
            .draw(&mut self.display)
            .unwrap();
    }

    pub async fn draw_temp(&mut self, temps: WeatherData) {
        info!("Draw temp");

        let mut text: String<50> = String::new();

        write!(&mut text, "{}°C", temps.temp).unwrap();
        draw_text(&mut self.display, text.as_str(), 97, 45);
    }

    pub async fn draw_weather_icon(&mut self, weather_code: &ConditionCode) {
        let icon_file = ConditionCode::icon(weather_code);
        let bmp_data = ICONS.iter().find(|item| item.0 == icon_file).unwrap().1;
        let bmp = Bmp::<'_, BinaryColor>::from_slice(bmp_data).unwrap();
        //Trait bound or error or sum is killing me so lemme just transform the color
        let offset = Point::new(15, 30);

        for Pixel(point, color) in bmp.pixels() {
            let display_color: Color = match color.into() {
                Color::Black => Color::White,
                Color::White => Color::Black,
            };
            Pixel(point + offset, display_color)
                .draw(&mut self.display)
                .unwrap();
        }

        Line::new(Point::new(80, 30), Point::new(80, 70))
            .into_styled(PrimitiveStyle::with_stroke(Color::Black, 5))
            .draw(&mut self.display)
            .unwrap();
    }
}

fn draw_text(display: &mut Display2in13, text: &str, x: i32, y: i32) {
    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_10X20)
        .text_color(Color::Black)
        .build();

    Text::with_baseline(text, Point::new(x, y), text_style, Baseline::Top)
        .draw(display)
        .unwrap();
}

fn month_name(month: u32) -> &'static str {
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "Err",
    }
}
