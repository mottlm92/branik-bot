use reqwest::Error;

pub struct PriceReader {
}

impl PriceReader {
    const URL: &'static str = "https://www.akcniceny.cz/akce/branik-pivo-vycepni-svetle-2-0l-pet/";

    pub async fn load_and_parse_branik_price(&self, default_price: f32) -> Result<f32, Error> {
        let return_default = |s: &str| {
            println!("Returning default price!");
            println!("{}", s);
            default_price
        };
        let text = if let Ok(text) = reqwest::get(Self::URL).await?.text().await {
            text
        } else {
            return Ok(return_default("Failed to load page as text"));
        };
        let low_price_line = if let Some(line) = text.lines().find(|l| l.contains("lowPrice")) {
            line
        } else {
            return Ok(return_default("Failed to find the \'lowPrice\' line"));
        };
        let end = if let Some(end) = low_price_line.find('>') {
            end
        } else {
            return Ok(return_default(
                "Failed to find enclosing element of the \'lowPrice\' containing span",
            ));
        };
        if let Some(content) = low_price_line[..=end]
            .split(" ")
            .find(|l| l.contains("content"))
        {
            let price = if let Some(p) = content.split("\"").nth(1) {
                p
            } else {
                return Ok(return_default(
                    "Failed to find String of price at index 1 in the split",
                ));
            };
            let price = if let Ok(p) = price.parse::<f32>() {
                p
            } else {
                return Ok(return_default("Failed to parse the price"));
            };
            return Ok(price);
        }
        Ok(return_default(
            "Failed to extract the \'lowPrice\' containing span",
        ))
    }
}
