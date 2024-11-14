use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Instant, Duration};
use csv::Writer;
use plotters::prelude::*;

/// Calculates the Bandwidth-Delay Product (BDP)
/// 
/// BDP represents the maximum amount of data (in bits) that can be in transit in the network.
/// 
/// # Arguments
/// - `bandwidth_bps`: The network bandwidth in bits per second (bps).
/// - `rtt_seconds`: The round-trip time (RTT) in seconds.
///
/// # Returns
/// - The Bandwidth-Delay Product in bits.
fn calculate_bdp(bandwidth_bps: f64, rtt_seconds: f64) -> f64 {
    bandwidth_bps * rtt_seconds
}

/// Calculates the Effective Data Rate, which represents the average data rate achieved over the entire transfer.
/// This takes into account the total data transferred and the total time taken.
///
/// # Arguments
/// - `total_data_bits`: Total amount of data transferred, in bits.
/// - `total_time_seconds`: Total time taken for the transfer, in seconds.
///
/// # Returns
/// - The Effective Data Rate in bits per second.
fn calculate_effective_data_rate(total_data_bits: f64, total_time_seconds: f64) -> f64 {
    total_data_bits / total_time_seconds
}

/// Calculates the TCP Throughput, which is typically limited by the BDP in networks with high latency.
/// This considers the size of the congestion window and RTT.
///
/// # Arguments
/// - `window_size_bits`: Size of the TCP congestion window in bits.
/// - `rtt_seconds`: The round-trip time (RTT) in seconds.
///
/// # Returns
/// - The TCP Throughput in bits per second.
fn calculate_tcp_throughput(window_size_bits: f64, rtt_seconds: f64) -> f64 {
    window_size_bits / rtt_seconds
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = TcpStream::connect("127.0.0.1:7878")?;
    println!("Connected to the server...");

    let mut buffer = vec![0u8; 1_000_000];
    let mut total_data_transferred = 0;
    let mut total_time = Duration::new(0, 0);
    let chunk_size = buffer.len() as f64 * 8.0;

    let mut wtr = Writer::from_path("download_metrics.csv")?;
    wtr.write_record(&["Chunk", "Download Time (s)", "Effective Data Rate (bps)"])?;

    let mut latencies = Vec::new();
    let mut data_rates = Vec::new();

    for i in 1..=100 {
        let start = Instant::now();
        stream.read_exact(&mut buffer)?;

        let duration = start.elapsed();
        total_time += duration;
        total_data_transferred += buffer.len();

        let download_time = duration.as_secs_f64();
        let effective_data_rate = chunk_size / download_time;

        latencies.push(download_time);
        data_rates.push(effective_data_rate);

        wtr.write_record(&[i.to_string(), download_time.to_string(), effective_data_rate.to_string()])?;
        println!("Chunk {}: Download Time: {:.2}s, Effective Data Rate: {:.2} bps", i, download_time, effective_data_rate);
    }

    wtr.flush()?;
    println!("Download metrics saved to download_metrics.csv");

    let total_data_bits = total_data_transferred as f64 * 8.0;
    let total_time_seconds = total_time.as_secs_f64();
    let avg_effective_data_rate = calculate_effective_data_rate(total_data_bits, total_time_seconds);
    let rtt_seconds = 0.2;
    let bdp = calculate_bdp(avg_effective_data_rate, rtt_seconds);
    let tcp_window_size_bits = 64_000.0 * 8.0;
    let tcp_throughput = calculate_tcp_throughput(tcp_window_size_bits, rtt_seconds);

    println!("Total Data Transferred: {:.2} MB", total_data_transferred as f64 / 1_000_000.0);
    println!("Average Effective Data Rate: {:.2} bps", avg_effective_data_rate);
    println!("Calculated BDP: {:.2} bits", bdp);
    println!("TCP Throughput: {:.2} bps", tcp_throughput);

    plot_latency_and_data_rate(&latencies, &data_rates)?;

    Ok(())
}

fn plot_latency_and_data_rate(latencies: &[f64], data_rates: &[f64]) -> Result<(), Box<dyn std::error::Error>> {
    let root = BitMapBackend::new("latency_data_rate.png", (1280, 960)).into_drawing_area();
    root.fill(&WHITE)?;

    let areas = root.split_evenly((2, 1));

    let avg_latency = latencies.iter().sum::<f64>() / latencies.len() as f64;
    let avg_data_rate = data_rates.iter().sum::<f64>() / data_rates.len() as f64;

    let smoothed_latencies: Vec<f64> = latencies.windows(5).map(|w| w.iter().sum::<f64>() / w.len() as f64).collect();
    let smoothed_data_rates: Vec<f64> = data_rates.windows(5).map(|w| w.iter().sum::<f64>() / w.len() as f64).collect();

    let mut latency_chart = ChartBuilder::on(&areas[0])
        .caption("Latency per Download (Smoothed)", ("sans-serif", 24).into_font())
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(1..smoothed_latencies.len() as i32, 0.0..smoothed_latencies.iter().cloned().fold(0./0., f64::max))?;
    
    latency_chart.configure_mesh()
        .x_desc("Download Number")
        .y_desc("Latency (s)")
        .y_label_formatter(&|y| format!("{:.5}", y))
        .axis_desc_style(("sans-serif", 14))
        .label_style(("sans-serif", 12))
        .light_line_style(&WHITE.mix(0.7))
        .draw()?;
    
    latency_chart.draw_series(LineSeries::new(
        (1..).zip(smoothed_latencies.iter().cloned()),
        &RED,
    ))?
    .label("Latency (s) (Smoothed)")
    .legend(|(x, y)| PathElement::new([(x - 5, y), (x + 5, y)], &RED));

    latency_chart.draw_series(std::iter::once(PathElement::new(
        [(1, avg_latency), (smoothed_latencies.len() as i32, avg_latency)], 
        RED.mix(0.5).stroke_width(2)
    )))?
    .label(format!("Avg Latency: {:.5} s", avg_latency))
    .legend(|(x, y)| PathElement::new([(x - 5, y), (x + 5, y)], RED.mix(0.5)));

    latency_chart.configure_series_labels()
        .border_style(&BLACK)
        .background_style(&WHITE.mix(0.8))
        .label_font(("sans-serif", 12))
        .draw()?;

    let mut data_rate_chart = ChartBuilder::on(&areas[1])
        .caption("Effective Data Rate per Download (Smoothed)", ("sans-serif", 24).into_font())
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(60)
        .build_cartesian_2d(1..smoothed_data_rates.len() as i32, 0.0..(avg_data_rate * 2.0))?;
    
    data_rate_chart.configure_mesh()
        .x_desc("Download Number")
        .y_desc("Data Rate (bps)")
        .y_label_formatter(&|y| format!("{:.2e}", y))
        .axis_desc_style(("sans-serif", 14))
        .label_style(("sans-serif", 12))
        .light_line_style(&WHITE.mix(0.7))
        .draw()?;
    
    data_rate_chart.draw_series(LineSeries::new(
        (1..).zip(smoothed_data_rates.iter().cloned()),
        &BLUE,
    ))?
    .label("Effective Data Rate (bps) (Smoothed)")
    .legend(|(x, y)| PathElement::new([(x - 5, y), (x + 5, y)], &BLUE));
    
    data_rate_chart.draw_series(std::iter::once(PathElement::new(
        [(1, avg_data_rate), (smoothed_data_rates.len() as i32, avg_data_rate)], 
        BLUE.mix(0.5).stroke_width(2)
    )))?
    .label(format!("Avg Data Rate: {:.2e} bps", avg_data_rate))
    .legend(|(x, y)| PathElement::new([(x - 5, y), (x + 5, y)], BLUE.mix(0.5)));

    data_rate_chart.configure_series_labels()
        .border_style(&BLACK)
        .background_style(&WHITE.mix(0.8))
        .label_font(("sans-serif", 12))
        .draw()?;

    println!("Refined Latency and Effective Data Rate chart saved as latency_data_rate.png");

    Ok(())
}
