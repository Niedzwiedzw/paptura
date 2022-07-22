mod directories;
mod filters;
mod slugify;
mod template_data;

use askama::Template;
use clap::{
    crate_version,
    App,
    Arg,
};
use std::io::BufRead;
use std::str::FromStr;
use std::{
    error::Error,
    num::ParseIntError,
};
use template_data::DaneFaktury;
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum PapturaError {
    #[error("Argument for value {0} was invalid")]
    BadArgumentFormat(String),
    #[error("Dodaj przynajmniej jeden przedmiot sprzedaży")]
    PrzedmiotSprzedazyMissing,
    #[error("Faktura o takiej nazwie już istnieje, prawdopodobnie coś poszło nietak")]
    DocumentAlreadyExists,
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
                .help(
                    "ścieżka do pliku YAML z configiem dla danego klienta (wyklucza się z --stdin)",
                ),
        )
        .arg(
            Arg::with_name("output-directory")
                .short("o")
                .long("output-directory")
                .value_name("DIR")
                .help("ścieżka do folderu w którym ma być zapisana faktura"),
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
        } else if let Some(config_path) = matches.value_of("config-path") {
            std::fs::read_to_string(config_path)?
        } else {
            panic!("--config-path FILE lub --stdin są wymagane")
        }
    };

    let mut dane_faktury: DaneFaktury =
        serde_json::from_str(config_str.as_str()).or(serde_yaml::from_str(config_str.as_str()))?;
    let output_directory =
        matches
            .value_of_os("output-directory")
            .ok_or(PapturaError::BadArgumentFormat(
                "output-directory is required".to_string(),
            ))?;
    let output_directory = std::path::PathBuf::from(output_directory);
    if !output_directory.exists() {
        return Err(Box::new(PapturaError::BadArgumentFormat(
            "output-directory doesn't exist".to_string(),
        )));
    }

    if !output_directory.is_dir() {
        return Err(Box::new(PapturaError::BadArgumentFormat(
            "output-directory needs to be a directory".to_string(),
        )));
    }
    if let Some(cena_netto) = matches.value_of("cena-netto") {
        dane_faktury
            .przedmiot_sprzedazy
            .first_mut()
            .ok_or(PapturaError::PrzedmiotSprzedazyMissing)?
            .cena_netto = rust_decimal::Decimal::from_str(cena_netto)
            .map_err(|e| PapturaError::BadArgumentFormat(e.to_string()))?
    }

    let slug = dane_faktury.slug();
    dane_faktury.numer_faktury = Some(
        dane_faktury.poczatek_serii_numeru_faktury
            + std::fs::read_dir(&output_directory)?
                .into_iter()
                .filter_map(|entry| entry.ok())
                .filter_map(|entry| entry.file_name().into_string().ok())
                .filter(|name| name.starts_with(slug.as_str()))
                .count() as u64,
    );
    let filename = dane_faktury.filename();
    let filepath = output_directory.join(filename);
    if filepath.exists() {
        return Err(Box::new(PapturaError::DocumentAlreadyExists));
    }
    std::fs::write(&filepath, dane_faktury.render()?)?;
    println!("{}", filepath.canonicalize()?.to_string_lossy());
    Ok(())
}
