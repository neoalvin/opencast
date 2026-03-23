use anyhow::Result;
use quick_xml::events::Event;
use quick_xml::Reader;
use reqwest::Client;
use tracing::debug;

/// Send a SOAP action to a UPnP service endpoint.
pub async fn soap_action(
    client: &Client,
    control_url: &str,
    service_type: &str,
    action: &str,
    body_xml: &str,
) -> Result<String> {
    let soap_envelope = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"
            s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
  <s:Body>
    <u:{action} xmlns:u="{service_type}">
      {body_xml}
    </u:{action}>
  </s:Body>
</s:Envelope>"#,
        action = action,
        service_type = service_type,
        body_xml = body_xml,
    );

    let soap_action_header = format!("\"{service_type}#{action}\"");

    debug!("SOAP request to {control_url}: {action}");

    let response = client
        .post(control_url)
        .header("Content-Type", "text/xml; charset=\"utf-8\"")
        .header("SOAPAction", &soap_action_header)
        .body(soap_envelope)
        .send()
        .await?;

    let status = response.status();
    let text = response.text().await?;

    if !status.is_success() {
        anyhow::bail!("SOAP action {action} failed with status {status}: {text}");
    }

    Ok(text)
}

/// Extract a named value from a SOAP response XML.
pub fn extract_xml_value(xml: &str, tag_name: &str) -> Option<String> {
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut in_target = false;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if name == tag_name || name.ends_with(&format!(":{tag_name}")) {
                    in_target = true;
                }
            }
            Ok(Event::Text(ref e)) if in_target => {
                return Some(e.unescape().unwrap_or_default().to_string());
            }
            Ok(Event::End(_)) => {
                in_target = false;
            }
            Ok(Event::Eof) => break,
            Err(_) => break,
            _ => {}
        }
        buf.clear();
    }
    None
}
