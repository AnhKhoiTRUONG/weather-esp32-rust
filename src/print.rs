use embassy_time::{Delay, Duration, Timer};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::gpio::{Input, InputConfig, Level, Output, OutputConfig, Pull};
use esp_println as _;

// SPI
use esp_hal::spi::master::Spi;

// epd
use epd_waveshare::epd2in13_v2::{Display2in13, Epd2in13};
use epd_waveshare::graphics::DisplayRotation;
use epd_waveshare::prelude::{Color, WaveshareDisplay};

// embedded graphics
use embedded_graphics::mono_font::MonoTextStyleBuilder;
use embedded_graphics::mono_font::ascii::FONT_10X20;
use embedded_graphics::prelude::*;
// use embedded_graphics::primitives::{Circle, PrimitiveStyle, PrimitiveStyleBuilder};
use embedded_graphics::text::{Baseline, Text};

use embassy_net::Stack;

use crate::api::{WeatherApi, WeatherData, WeatherResponse};

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

        self.epd
            .update_and_display_frame(&mut self.spi_dev, self.display.buffer(), &mut Delay)
            .unwrap();
        Timer::after(Duration::from_secs(5)).await;

        self.epd.sleep(&mut self.spi_dev, &mut Delay).unwrap();
    }

    pub async fn draw_date(&mut self, dt: DateTime<Utc>) {
        info!("Draw date");
        info!("Date: {:04}-{:02}-{:02}", dt.year(), dt.month(), dt.day());
        let mut text: String<50> = String::new();
        write!(
            &mut text,
            "Date: {:02}-{:02}-{:04}\nTime: {:02}:{:02}",
            dt.day(),
            dt.month(),
            dt.year(),
            dt.hour(),
            dt.minute(),
        )
        .unwrap();
        draw_text(&mut self.display, text.as_str(), 10, 0);
    }

    pub async fn draw_temp(&mut self, temps: WeatherData) {
        info!("Draw temp");

        let mut text: String<50> = String::new();

        write!(
            &mut text,
            "Temperature: {}\nFeels like: {}",
            temps.temp, temps.feels_like
        )
        .unwrap();
        draw_text(&mut self.display, text.as_str(), 10, 40);
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
