use crate::client;
use crate::output::csv::format_sources_csv;
use crate::output::format::SourcesFormat;
use crate::output::jsonl::format_sources_json;
use crate::output::table::format_sources_table;

pub async fn run(format: SourcesFormat) -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = client::connect().await?;
    let sources = client::list_sources(&mut stream, 1).await?;
    let output = match format {
        SourcesFormat::Table => format_sources_table(&sources),
        SourcesFormat::Csv => format_sources_csv(&sources),
        SourcesFormat::Json => format_sources_json(&sources),
    };

    println!("{output}");
    Ok(())
}
