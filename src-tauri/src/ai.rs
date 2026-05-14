use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::json;

use crate::models::{AiDecision, Classification, PrivacyMode, Settings, WindowInfo};

pub struct AiAnalyzer {
  client: Client,
}

impl AiAnalyzer {
  pub fn new() -> Self {
    Self { client: Client::new() }
  }

  pub async fn analyze(
    &self,
    task: &str,
    settings: &Settings,
    window: &WindowInfo,
    screenshot_data_url: Option<&str>,
  ) -> Result<Option<AiDecision>> {
    if !settings.ai_enabled || settings.privacy_mode == PrivacyMode::LocalOnly || settings.openai_api_key.trim().is_empty() {
      return Ok(None);
    }

    let mut content = vec![json!({
      "type": "input_text",
      "text": format!(
        "学习任务：{}\n前台应用：{}\n窗口标题：{}\n请判断这一刻是否在认真学习，只返回 JSON：{{\"classification\":\"focused|distracted|idle|unknown\",\"confidence\":0到1,\"reason\":\"中文短原因\",\"topic\":\"学习内容标签\"}}",
        task, window.app_name, window.window_title
      )
    })];

    if let Some(data_url) = screenshot_data_url {
      content.push(json!({
        "type": "input_image",
        "image_url": data_url,
        "detail": "low"
      }));
    }

    let body = json!({
      "model": settings.openai_model,
      "instructions": "你是一个严格但克制的自习监督分析器。只输出有效 JSON，不要输出 Markdown。",
      "input": [
        {
          "role": "user",
          "content": content
        }
      ],
      "temperature": 0
    });

    let endpoint = responses_endpoint(&settings.openai_api_base_url);
    let value: serde_json::Value = self
      .client
      .post(&endpoint)
      .bearer_auth(settings.openai_api_key.trim())
      .json(&body)
      .send()
      .await
      .context("OpenAI request failed")?
      .error_for_status()
      .context("OpenAI returned an error")?
      .json()
      .await
      .context("failed to parse OpenAI response")?;

    let text = extract_output_text(&value).context("OpenAI response did not contain output text")?;
    let parsed: serde_json::Value = serde_json::from_str(text.trim()).context("AI output was not valid JSON")?;
    let classification = parsed
      .get("classification")
      .and_then(|value| value.as_str())
      .unwrap_or("unknown")
      .to_string();
    Ok(Some(AiDecision {
      classification: Classification::from(classification),
      confidence: parsed.get("confidence").and_then(|value| value.as_f64()).unwrap_or(0.5) as f32,
      reason: parsed
        .get("reason")
        .and_then(|value| value.as_str())
        .unwrap_or("AI 已分析")
        .to_string(),
      topic: parsed
        .get("topic")
        .and_then(|value| value.as_str())
        .unwrap_or(task)
        .to_string(),
    }))
  }
}

fn extract_output_text(value: &serde_json::Value) -> Option<&str> {
  value.get("output_text").and_then(|item| item.as_str()).or_else(|| {
    value
      .get("output")?
      .as_array()?
      .iter()
      .flat_map(|item| item.get("content").and_then(|content| content.as_array()).into_iter().flatten())
      .find_map(|content| content.get("text").and_then(|text| text.as_str()))
  })
}

fn responses_endpoint(base_url: &str) -> String {
  let trimmed = base_url.trim().trim_end_matches('/');
  if trimmed.is_empty() {
    return "https://api.openai.com/v1/responses".to_string();
  }
  if trimmed.ends_with("/responses") {
    trimmed.to_string()
  } else {
    format!("{trimmed}/responses")
  }
}
