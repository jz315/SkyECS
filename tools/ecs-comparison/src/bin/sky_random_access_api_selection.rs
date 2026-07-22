use sky_ecs_comparison::sky::certify_random_access_apis;

fn main() {
    let certification = certify_random_access_apis(4);
    println!(
        "{}",
        serde_json::to_string_pretty(&certification)
            .expect("Sky random-access API certification result must serialize")
    );
}
