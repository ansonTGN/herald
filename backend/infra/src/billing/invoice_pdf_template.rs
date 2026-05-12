use herald_domain::billing::invoice::InvoiceDetail;

/// Render an InvoiceDetail to an HTML string suitable for PDF conversion.
/// All dynamic text is HTML-escaped to prevent injection.
pub fn render_invoice_html(detail: &InvoiceDetail) -> String {
    let inv = &detail.invoice;

    let invoice_number = html_escape(&inv.invoice_number);
    let status = html_escape(inv.status.as_str());
    let currency = html_escape(&inv.currency);

    let billing_name = html_escape(&inv.billing_name);
    let billing_address = opt_div(&inv.billing_address);
    let billing_email = opt_div(&inv.billing_email);
    let billing_phone = opt_div(&inv.billing_phone);

    let seller_name = html_escape(&inv.seller_name);
    let seller_address = opt_div(&inv.seller_address);
    let seller_email = opt_div(&inv.seller_email);
    let seller_phone = opt_div(&inv.seller_phone);

    let issue_date = inv
        .issue_date
        .map_or(String::new(), |d| html_escape(&d.to_string()));
    let due_date = inv
        .due_date
        .map_or(String::new(), |d| html_escape(&d.to_string()));
    let payment_terms = inv
        .payment_terms
        .as_deref()
        .map_or(String::new(), html_escape);

    let subtotal = format_cents(inv.subtotal);
    let discount = format_cents(inv.discount_amount);
    let tax = format_cents(inv.tax_amount);
    let shipping = format_cents(inv.shipping_amount);
    let total = format_cents(inv.total);

    let notes = inv.notes.as_deref().map_or(String::new(), |s| {
        format!(
            "<div class=\"notes\"><strong>Notes:</strong> {}</div>",
            html_escape(s)
        )
    });

    let mut line_items_html = String::new();
    for item in &detail.line_items {
        let name = html_escape(&item.name);
        let desc = item.description.as_deref().map_or(String::new(), |d| {
            format!("<br/><small>{}</small>", html_escape(d))
        });
        let qty = html_escape(&item.quantity);
        let unit_price = format_cents(item.unit_price);
        let item_subtotal = format_cents(item.subtotal);
        line_items_html.push_str(&format!(
            "<tr><td>{}{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            name, desc, qty, unit_price, item_subtotal
        ));
    }

    let discount_row = if inv.discount_amount != 0 {
        format!(
            "<tr><td colspan=\"3\">Discount</td><td>{}</td></tr>",
            discount
        )
    } else {
        String::new()
    };

    let tax_row = if inv.tax_amount != 0 {
        format!("<tr><td colspan=\"3\">Tax</td><td>{}</td></tr>", tax)
    } else {
        String::new()
    };

    let shipping_row = if inv.shipping_amount != 0 {
        format!(
            "<tr><td colspan=\"3\">Shipping</td><td>{}</td></tr>",
            shipping
        )
    } else {
        String::new()
    };

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8"/>
<style>
  body {{ font-family: sans-serif; font-size: 12px; color: #333; margin: 0; padding: 20px; }}
  h1 {{ font-size: 20px; margin-bottom: 4px; }}
  .header {{ display: flex; justify-content: space-between; margin-bottom: 24px; }}
  .parties {{ display: flex; justify-content: space-between; margin-bottom: 24px; }}
  .party {{ width: 45%; }}
  .party h3 {{ font-size: 13px; margin-bottom: 4px; color: #666; }}
  .party div {{ margin: 2px 0; }}
  table {{ width: 100%; border-collapse: collapse; margin-bottom: 16px; }}
  th {{ text-align: left; padding: 8px; border-bottom: 2px solid #333; font-size: 11px; }}
  td {{ padding: 8px; border-bottom: 1px solid #ddd; }}
  .totals {{ float: right; width: 300px; }}
  .totals td {{ padding: 4px 8px; }}
  .totals .total-row {{ font-weight: bold; font-size: 14px; border-top: 2px solid #333; }}
  .notes {{ margin-top: 24px; padding: 12px; background: #f9f9f9; border-radius: 4px; }}
  .status {{ display: inline-block; padding: 2px 8px; background: #e0e0e0; border-radius: 3px; font-size: 11px; text-transform: uppercase; }}
  .clearfix {{ clear: both; }}
</style>
</head>
<body>
  <div class="header">
    <div>
      <h1>INVOICE</h1>
      <div>{}</div>
      <div class="status">{}</div>
    </div>
    <div style="text-align: right;">
      <div><strong>Date:</strong> {}</div>
      <div><strong>Due:</strong> {}</div>
      <div><strong>Terms:</strong> {}</div>
    </div>
  </div>

  <div class="parties">
    <div class="party">
      <h3>From</h3>
      <div><strong>{}</strong></div>
      {}{}{}
    </div>
    <div class="party">
      <h3>Bill To</h3>
      <div><strong>{}</strong></div>
      {}{}{}
    </div>
  </div>

  <table>
    <thead>
      <tr><th>Description</th><th>Qty</th><th>Unit Price</th><th>Amount</th></tr>
    </thead>
    <tbody>
      {}
    </tbody>
  </table>

  <div class="totals">
    <table>
      <tr><td>Subtotal</td><td>{}</td></tr>
      {}{}{}
      <tr class="total-row"><td>Total ({})</td><td>{}</td></tr>
    </table>
  </div>
  <div class="clearfix"></div>

  {}
</body>
</html>"#,
        invoice_number,
        status,
        issue_date,
        due_date,
        payment_terms,
        seller_name,
        seller_address,
        seller_email,
        seller_phone,
        billing_name,
        billing_address,
        billing_email,
        billing_phone,
        line_items_html,
        subtotal,
        discount_row,
        tax_row,
        shipping_row,
        currency,
        total,
        notes,
    )
}

/// Format a cents value as a decimal string (e.g., 1234 -> "12.34").
fn format_cents(cents: i64) -> String {
    let abs = cents.unsigned_abs();
    let whole = abs / 100;
    let frac = abs % 100;
    if cents < 0 {
        format!("-{}.{:02}", whole, frac)
    } else {
        format!("{}.{:02}", whole, frac)
    }
}

/// Escape characters that are special in HTML.
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Wrap an optional string in a `<div>` with HTML escaping.
fn opt_div(val: &Option<String>) -> String {
    val.as_deref()
        .map_or(String::new(), |s| format!("<div>{}</div>", html_escape(s)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_escape_escapes_special_chars() {
        assert_eq!(
            html_escape("<script>alert('x')</script>"),
            "&lt;script&gt;alert(&#39;x&#39;)&lt;/script&gt;"
        );
        assert_eq!(html_escape("a & b"), "a &amp; b");
        assert_eq!(html_escape("say \"hello\""), "say &quot;hello&quot;");
    }

    #[test]
    fn format_cents_positive() {
        assert_eq!(format_cents(1234), "12.34");
        assert_eq!(format_cents(100), "1.00");
        assert_eq!(format_cents(0), "0.00");
        assert_eq!(format_cents(99), "0.99");
    }

    #[test]
    fn format_cents_negative() {
        assert_eq!(format_cents(-1234), "-12.34");
        assert_eq!(format_cents(-100), "-1.00");
    }
}
