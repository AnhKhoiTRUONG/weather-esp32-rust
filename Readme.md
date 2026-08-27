## Weather station in Rust

Why? Because weather station is cool, Rust is cool and E-ink display is cool

![The result](/display.jpg)

- It updates every 10 minutes
- A nice icon for every weather (rainy, sunny, thunderstorm, ...)

### What you need for this project?
- An ESP32 module (that I bought on Aliexpress)
- A WeAct 2.13" Epaper Module (also on Aliexpress)


### Wifi and API
- So here I use the ![Openweather API](https://openweathermap.org/) so to run it so you need to put a `.env` file in the main directory
```conf
SSID = 
PASS = 
API = 
```

### Dependencies
This project use the `epd-waveshare` crate, but we have some small problems because our display is not a Waveshare display so I made a few changes in the `epd-waveshare`, so u need to clone this ![fork](https://github.com/AnhKhoiTRUONG/epd-waveshare.git) for it to work.
