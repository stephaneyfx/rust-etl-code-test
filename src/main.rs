use anyhow::Context;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    ops::Add,
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Record {
    name: String,
    billing_code: String,
    #[serde(
        deserialize_with = "serdapt::From::<AccumulatedRate, serdapt::Fold<NegotiatedRate, AccumulatedRate>>::deserialize",
        rename(deserialize = "negotiated_rates")
    )]
    avg_rate: Option<f64>,
}

#[derive(Debug, Default)]
struct AccumulatedRate {
    rate: f64,
    count: u64,
}

impl From<AccumulatedRate> for Option<f64> {
    fn from(value: AccumulatedRate) -> Self {
        if value.count == 0 {
            None
        } else {
            Some(value.rate / value.count as f64)
        }
    }
}

impl Add<NegotiatedRate> for AccumulatedRate {
    type Output = Self;

    fn add(self, rhs: NegotiatedRate) -> Self::Output {
        Self {
            rate: self.rate + rhs.negotiated_prices.rate,
            count: self.count + rhs.negotiated_prices.count,
        }
    }
}

impl Add<NegotiatedPrice> for AccumulatedRate {
    type Output = Self;

    fn add(self, rhs: NegotiatedPrice) -> Self::Output {
        Self {
            rate: self.rate + rhs.negotiated_rate,
            count: self.count + 1,
        }
    }
}

#[derive(Debug, Deserialize)]
struct NegotiatedRate {
    #[serde(with = "serdapt::Fold::<NegotiatedPrice, AccumulatedRate>")]
    negotiated_prices: AccumulatedRate,
}

#[derive(Debug, Deserialize)]
struct NegotiatedPrice {
    negotiated_rate: f64,
}

/// Extract billing information from JSONL input and outputs records in CSV format
#[derive(Debug, Parser)]
struct Cli {
    /// Input file to read JSONL from (defaults to stdin)
    #[arg(short, long)]
    input: Option<PathBuf>,
    /// Output file to write CSV to (defaults to stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match (cli.input, cli.output) {
        (None, None) => process(std::io::stdin().lock(), std::io::stdout().lock()),
        (None, Some(output)) => process(std::io::stdin().lock(), open_output(&output)?),
        (Some(input), None) => process(open_input(&input)?, std::io::stdout().lock()),
        (Some(input), Some(output)) => process(open_input(&input)?, open_output(&output)?),
    }
}

fn open_input(p: &Path) -> anyhow::Result<BufReader<File>> {
    Ok(BufReader::new(File::open(p).with_context(|| {
        format!("failed to open {}", p.display())
    })?))
}

fn open_output(p: &Path) -> anyhow::Result<BufWriter<File>> {
    Ok(BufWriter::new(File::create(p).with_context(|| {
        format!("failed to open {}", p.display())
    })?))
}

fn process<I, O>(input: I, output: O) -> anyhow::Result<()>
where
    I: BufRead,
    O: Write,
{
    let mut output = csv::Writer::from_writer(output);
    for (i, r) in records(input).enumerate() {
        let r = r.with_context(|| format!("error on line {}", i + 1))?;
        if r.avg_rate.is_some_and(|r| r <= 30.0) {
            output.serialize(r).context("failed to write record")?;
        }
    }
    output.flush()?;
    Ok(())
}

fn records<I>(input: I) -> impl Iterator<Item = anyhow::Result<Record>>
where
    I: BufRead,
{
    input.lines().map(|line| {
        let line = line.context("failed to read line")?;
        serde_json::from_str(&line).context("failed to parse record")
    })
}

#[cfg(test)]
mod tests {
    use crate::Record;
    use serde_json::json;

    #[test]
    fn average_is_calculated() {
        let input = json!({
            "name": "alpha",
            "billing_code": "1",
            "negotiated_rates": [
                {
                    "negotiated_prices": [
                        {
                            "negotiated_rate": 10,
                        },
                    ],
                },
                {
                    "negotiated_prices": [],
                },
                {
                    "negotiated_prices": [
                        {
                            "negotiated_rate": 20,
                        },
                        {
                            "negotiated_rate": 60,
                        },
                    ],
                },
            ],
        });

        let expected = Record {
            name: "alpha".into(),
            billing_code: "1".into(),
            avg_rate: Some(30.0),
        };

        let actual = serde_json::from_value::<Record>(input).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn average_is_none_when_no_rates() {
        let input = json!({
            "name": "alpha",
            "billing_code": "1",
            "negotiated_rates": [],
        });

        let actual = serde_json::from_value::<Record>(input).unwrap();
        assert_eq!(actual.avg_rate, None);
    }
}
