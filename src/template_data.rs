use crate::filters;
use askama::Template;
use chrono::Datelike;
use chrono::NaiveDate;
use rust_decimal::prelude::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use clap::crate_version;

#[derive(Serialize, Deserialize, Debug)]
pub struct KontoBankowe {
    przedrostek_banku: String,
    numer_konta: String,
}

impl Default for KontoBankowe {
    fn default() -> Self {
        Self {
            przedrostek_banku: "PKO BP".to_string(),
            numer_konta: "21 2137 2137 2137 2137 2137 2137".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Adres {
    adres_1: String,
    adres_2: String,
}

impl Default for Adres {
    fn default() -> Self {
        Self {
            adres_1: "Kremóweczki-Małe, ul. Janusza Pawlacza 21/37".to_string(),
            adres_2: "21-370 Kremówkowice".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StronaSprzedazy {
    nazwa: String,
    nip: u64,
    konto_bankowe: Option<KontoBankowe>,
    adres: Adres,
}

impl Default for StronaSprzedazy {
    fn default() -> Self {
        Self {
            nazwa: "Papaj - Janusz Pawlacz".to_string(),
            nip: 8911632619,
            konto_bankowe: Some(KontoBankowe::default()),
            adres: Adres::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PrzedmiotSprzedazy {
    nazwa: String,
    jednostka: String,
    ilosc: Decimal,
    pub cena_netto: Decimal,
    stawka: Decimal,
}

impl Default for PrzedmiotSprzedazy {
    fn default() -> Self {
        Self {
            nazwa: "wytwarzanie oprogramowania stopniowo zwiększającego zawartość alkoholu w kremówkach"
                .to_string(),
            jednostka: "szt.".to_string(),
            ilosc: dec!(1.00),
            cena_netto: dec!(2137.00),
            stawka: dec!(0.23),
        }
    }
}


impl PrzedmiotSprzedazy {
    fn wartosc_netto(&self) -> Decimal {
        self.cena_netto
    }

    fn wartosc_brutto(&self) -> Option<Decimal> {
        self.wartosc_netto().checked_add(self.kwota_vat()?)
    }

    fn kwota_vat(&self) -> Option<Decimal> {
        self.wartosc_netto().checked_mul(self.stawka)
    }
}

/// based on https://pl.wikipedia.org/wiki/Faktura_(dokument)
#[derive(Serialize, Deserialize, Debug, Template)]
#[template(path = "faktura.html")]
pub struct DaneFaktury {
    pub numer_faktury: Option<u64>,
    metoda_platnosci: String,
    sprzedawca: StronaSprzedazy,
    nabywca: StronaSprzedazy,
    pub przedmiot_sprzedazy: Vec<PrzedmiotSprzedazy>,
    zaplacono: Decimal,
    uwagi: String,
}

pub fn today() -> NaiveDate {
    chrono::Local::today().naive_local()
}

impl Default for DaneFaktury {
    fn default() -> Self {
        Self {
            numer_faktury: None,
            sprzedawca: StronaSprzedazy::default(),
            nabywca: StronaSprzedazy::default(),
            przedmiot_sprzedazy: vec![PrzedmiotSprzedazy::default()],
            zaplacono: dec!(0.00),
            uwagi: "GTU_12".to_string(),
            metoda_platnosci: "przelew".to_string(),
        }
    }
}

impl DaneFaktury {
    fn numer_faktury(&self) -> u64 {
        self.numer_faktury.unwrap_or(self.data_wystawienia().month() as u64)
    }
    fn data_wystawienia(&self) -> NaiveDate {
        today()
    }
    fn data_sprzedazy(&self) -> NaiveDate {
        self.data_wystawienia()
    }

    fn termin_platnosci(&self) -> NaiveDate {
        self.data_wystawienia() + chrono::Duration::days(3)
    }

    fn wartosc_netto(&self) -> Option<Decimal> {
        if self.przedmiot_sprzedazy.is_empty() {
            return None;
        }
        Some(
            self.przedmiot_sprzedazy
                .iter()
                .map(|p| p.wartosc_netto())
                .sum(),
        )
    }

    fn wartosc_brutto(&self) -> Option<Decimal> {
        let wartosci_brutto_sprzedazy = self.przedmiot_sprzedazy.iter().filter_map(|p| p.wartosc_brutto()).collect::<Vec<_>>();
        if wartosci_brutto_sprzedazy.is_empty() {
            return None;
        }

        Some(
            wartosci_brutto_sprzedazy
                .iter()
                .sum(),
        )
    }

    fn kwota_vat(&self) -> Option<Decimal> {
        let kwoty_vat = self.przedmiot_sprzedazy.iter().filter_map(|p| p.kwota_vat()).collect::<Vec<_>>();
        if kwoty_vat.is_empty() {
            return None;
        }

        Some(
            kwoty_vat.iter().sum()
        )
    }

    fn wersja(&self) -> String {
        format!("https://github.com/Niedzwiedzw/faktura (wersja {})", crate_version!())
    }
}
