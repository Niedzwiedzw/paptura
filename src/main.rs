mod directories;
mod filters;
mod template_data;

use askama::Template;
use clap::{crate_version, App, Arg, ArgMatches};
use std::io::BufRead;
use std::str::FromStr;
use std::{error::Error, num::ParseIntError};
use template_data::DaneFaktury;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum PapturaError {
    #[error("Argument for value {0} was invalid")]
    BadArgumentFormat(String),
    #[error("Dodaj przynajmniej jeden przedmiot sprzedaży")]
    PrzedmiotSprzedazyMissing,
}

fn main() -> Result<(), Box<dyn Error>> {
    let matches: clap::ArgMatches = App::new("paptura")
        .version(crate_version!())
        .author("Wojciech Niedźwiedź. <wojciech.brozek@niedzwiedz.it>")
        .about("Wystawianie faktur bez badziewia")
        .arg(
            Arg::with_name("example-config")
                .short("e")
                .long("example-config")
                .help("wypisuje przykładowy config w YAMLu"),
        )
        .arg(
            Arg::with_name("cena-netto")
                .short("c")
                .long("cena-netto")
                .value_name("PIENIĄDZE")
                .help("nadpisuje cenę netto towaru / usługi, np '2137.99'"),
        )
        .arg(
            Arg::with_name("numer-faktury")
                .short("n")
                .long("numer-faktury")
                .value_name("numer")
                .help("nadpisuje numer faktury (domyślnie numer miesiąca)"),
        )
        .arg(
            Arg::with_name("stdin")
                .short("s")
                .long("stdin")
                .help("określa czy config ma byc pobrany z STDIN"),
        )
        .arg(
            Arg::with_name("config-path")
                .short("p")
                .long("config-path")
                .value_name("FILE")
                .help("ścieżka do pliku YAML z configiem dla danego klienta (wyklucza się z --stdin)"),
        )
        .get_matches();
    if matches.is_present("example-config") {
        println!("{}", serde_yaml::to_string(&DaneFaktury::default())?);
        std::process::exit(0);
    }

    let stdin = std::io::stdin();
    let config_str = {
        if matches.is_present("stdin") {
            stdin
                .lock()
                .lines()
                .filter_map(|line| line.ok())
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            if let Some(config_path) = matches.value_of("config-path") {
                std::fs::read_to_string(config_path)?
            } else {
                panic!("--config-path FILE lub --stdin są wymagane".to_string())
            }
        }


    };

    let mut dane_faktury: DaneFaktury =
        serde_json::from_str(config_str.as_str()).or(serde_yaml::from_str(config_str.as_str()))?;

    if let Some(cena_netto) = matches.value_of("cena-netto") {
        dane_faktury
            .przedmiot_sprzedazy
            .first_mut()
            .ok_or(PapturaError::PrzedmiotSprzedazyMissing)?
            .cena_netto = rust_decimal::Decimal::from_str(cena_netto)
            .map_err(|e| PapturaError::BadArgumentFormat(e.to_string()))?
    }

    if let Some(numer_faktury) = matches.value_of("numer-faktury") {
        dane_faktury.numer_faktury = numer_faktury
            .parse()
            .map_err(|e: ParseIntError| PapturaError::BadArgumentFormat(e.to_string()))?
    }


    println!("{}", dane_faktury.render()?);
    Ok(())
}
