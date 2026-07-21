use sky_ecs_comparison::sky::certify_gameplay_apis;

fn main() {
    let certification = certify_gameplay_apis(4, 16);
    println!(
        "{}",
        serde_json::to_string_pretty(&certification)
            .expect("API certification result must serialize")
    );
}
