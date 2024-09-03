use std::time::Instant;
use futures::future::join_all;
use tokio::fs;
use tracing::info;
use website::process_website;

mod favicon;
mod website;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    #[cfg(debug_assertions)]
    let start_time = Instant::now(); // Start the timer only in debug mode

    // Read the JSON file containing the list of websites
    let data = fs::read_to_string("websites.json").await?;
    let website_list: website::WebsiteList = serde_json::from_str(&data)?;

    // Create a vector of futures for processing each website
    let tasks: Vec<_> = website_list.0
        .iter()
        .map(|website| process_website(website))
        .collect();

    // Run all tasks concurrently using join_all
    let results = join_all(tasks).await;

    // Handle any errors (optional)
    for result in results {
        if let Err(e) = result {
            eprintln!("Error processing website: {}", e);
        }
    }

    #[cfg(debug_assertions)]
    {
        // Stop the timer and print the duration in debug mode
        let duration = start_time.elapsed();
        info!("Time elapsed: {:?}", duration);
    }

    Ok(())
}
