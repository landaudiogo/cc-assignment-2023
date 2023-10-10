use reqwest::header::CONTENT_TYPE;
use reqwest::header::ACCEPT;
use reqwest::Response;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let res = client
        .post("http://127.0.0.1:3000/api/notify")
        .header(CONTENT_TYPE, "application/json; charset=utf-8")
        .header(ACCEPT, "text/plain; chaset=utf-8")
        .body(r#"{
            "notification_type": "OutOfRange",
            "researcher": "d.landau@uu.nl",
            "measurement_id": "1234",
            "experiment_id": "5678",
            "cipher_data": "D5qnEHeIrTYmLwYX.hSZNb3xxQ9MtGhRP7E52yv2seWo4tUxYe28ATJVHUi0J++SFyfq5LQc0sTmiS4ILiM0/YsPHgp5fQKuRuuHLSyLA1WR9YIRS6nYrokZ68u4OLC4j26JW/QpiGmAydGKPIvV2ImD8t1NOUrejbnp/cmbMDUKO1hbXGPfD7oTvvk6JQVBAxSPVB96jDv7C4sGTmuEDZPoIpojcTBFP2xA"
        }"#)
        .send()
        .await
        .unwrap();
    println!("{:?}", res.text_with_charset("utf-8").await);
    Ok(())
}

