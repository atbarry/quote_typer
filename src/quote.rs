use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Quote {
    // #[serde(rename = "_id")]
    // pub id: String,
    pub content: String,
    pub author: String,
    pub tags: Vec<String>,
    // pub author_slug: String,
    pub length: i64,
    // pub date_added: String,
    // pub date_modified: String,
}

impl Quote {
    pub fn content_chars(&self) -> Vec<char> {
        self.content.chars().collect()
    }
}

pub async fn get_quote() -> Result<Quote, reqwest::Error> {
    reqwest::get("https://api.quotable.io/random")
        .await?
        .json::<Quote>()
        .await
}
