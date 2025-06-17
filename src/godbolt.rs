use reqwest::Error;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Language {
    pub id: String,
    pub name: String
}

pub async fn languages() -> Result<Vec<Language>, Error> {
    let request_url = "https://godbolt.org/api/languages";
    let client = reqwest::Client::new();
    let res = client
        .get(format!("{request_url}"))
        .header("Accept", "application/json")
        .send()
        .await?;
    let langs : Vec<Language> = res.json().await?;

    Ok(langs)
}
