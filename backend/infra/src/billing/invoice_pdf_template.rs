use herald_domain::billing::invoice::InvoiceDetail;

const COLOR_PRIMARY: &str = "#1a365d";
const COLOR_PRIMARY_LIGHT: &str = "#2b6cb0";
const COLOR_BG_LIGHT: &str = "#f7fafc";
const COLOR_BORDER: &str = "#e2e8f0";
const COLOR_MUTED: &str = "#718096";
const COLOR_SUBTLE: &str = "#cbd5e0";

/// Render an InvoiceDetail to an HTML string suitable for PDF conversion.
/// All dynamic text is HTML-escaped to prevent injection.
pub fn render_invoice_html(detail: &InvoiceDetail) -> String {
    let inv = &detail.invoice;

    let invoice_number = html_escape(&inv.invoice_number);
    let status = html_escape(inv.status.as_str());
    let currency = html_escape(&inv.currency);

    let billing_name = html_escape(inv.billing_name.as_deref().unwrap_or(""));
    let billing_address = inv
        .billing_address
        .as_deref()
        .map_or(String::new(), |addr| {
            format!("<div>{}</div>", html_escape(addr))
        });
    let billing_email = opt_div(&inv.billing_email);
    let billing_phone = opt_div(&inv.billing_phone);
    let billing_tax_id = opt_div(&inv.billing_tax_id);

    let seller_name = html_escape(inv.seller_name.as_deref().unwrap_or(""));
    let seller_address = inv.seller_address.as_deref().map_or(String::new(), |addr| {
        format!("<div>{}</div>", html_escape(addr))
    });
    let seller_email = opt_div(&inv.seller_email);
    let seller_phone = opt_div(&inv.seller_phone);
    let seller_tax_id = opt_div(&inv.seller_tax_id);

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
            "<div style=\"margin-top: 24px; padding: 12px; background: #f7fafc; border-top: 2px solid #e2e8f0;\"><strong>Additional Information:</strong> {}</div>",
            html_escape(s)
        )
    });

    let status_color = match inv.status.as_str() {
        "paid" => "#16a34a",
        "issued" => "#2563eb",
        "overdue" => "#d97706",
        "draft" => "#6b7280",
        "void" => "#dc2626",
        _ => "#6b7280",
    };

    let mut line_items_html = String::new();
    for (i, item) in detail.line_items.iter().enumerate() {
        let name = html_escape(&item.name);
        let desc = item.description.as_deref().map_or(String::new(), |d| {
            format!("<br/><small>{}</small>", html_escape(d))
        });
        let qty = html_escape(&item.quantity);
        let unit_price = format_cents(item.unit_price);
        let item_subtotal = format_cents(item.subtotal);
        let row_bg = if i % 2 == 1 {
            " style=\"background: #f9fafb;\""
        } else {
            ""
        };
        line_items_html.push_str(&format!(
            "<tr{}><td>{}{}</td><td style=\"text-align: right;\">{}</td><td style=\"text-align: right;\">{}</td><td style=\"text-align: right;\">{}</td></tr>",
            row_bg, name, desc, qty, unit_price, item_subtotal
        ));
    }

    let discount_row = if inv.discount_amount != 0 {
        format!(
            "<tr><td style=\"padding: 4px 8px;\">Discount</td><td style=\"padding: 4px 8px; text-align: right;\">{}</td></tr>",
            discount
        )
    } else {
        String::new()
    };

    let tax_row = if inv.tax_amount != 0 {
        format!(
            "<tr><td style=\"padding: 4px 8px;\">Tax</td><td style=\"padding: 4px 8px; text-align: right;\">{}</td></tr>",
            tax
        )
    } else {
        String::new()
    };

    let shipping_row = if inv.shipping_amount != 0 {
        format!(
            "<tr><td style=\"padding: 4px 8px;\">Shipping</td><td style=\"padding: 4px 8px; text-align: right;\">{}</td></tr>",
            shipping
        )
    } else {
        String::new()
    };

    let th_style = format!(
        "padding: 8px; background: {COLOR_PRIMARY}; color: #ffffff; font-size: 11px; font-weight: bold;"
    );

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8"/>
<style>
  body {{ font-family: sans-serif; font-size: 12px; color: #333; margin: 0; padding: 40px; }}
  table {{ border-collapse: collapse; }}
  td {{ padding: 0; }}
</style>
</head>
<body>

<table width="100%" style="margin-bottom: 20px;">
  <tr>
    <td style="background: {COLOR_PRIMARY}; padding: 16px 24px;" width="100%">
      <table width="100%">
        <tr>
          <td>
            <div style="font-size: 24px; font-weight: bold; color: #ffffff;">INVOICE</div>
            <div style="font-size: 13px; color: {COLOR_SUBTLE}; margin-top: 2px;">{}</div>
          </td>
          <td style="text-align: right;">
            <span style="background: {}; color: #ffffff; padding: 4px 12px; font-size: 11px; font-weight: bold; text-transform: uppercase;">{}</span>
          </td>
        </tr>
      </table>
    </td>
  </tr>
</table>

<table width="100%" style="margin-bottom: 24px;">
  <tr>
    <td style="padding-right: 8px;" width="50%">
      <div style="padding: 12px; background: {COLOR_BG_LIGHT};\">
        <div style="font-size: 11px; font-weight: bold; color: {COLOR_PRIMARY_LIGHT}; text-transform: uppercase; margin-bottom: 8px;">From</div>
        <div style="font-weight: bold; margin-bottom: 4px;">{}</div>
        {}{}{}{}
      </div>
    </td>
    <td style="padding-left: 8px;" width="50%">
      <div style="padding: 12px; background: {COLOR_BG_LIGHT};\">
        <div style="font-size: 11px; font-weight: bold; color: {COLOR_PRIMARY_LIGHT}; text-transform: uppercase; margin-bottom: 8px;">Bill To</div>
        <div style="font-weight: bold; margin-bottom: 4px;">{}</div>
        {}{}{}{}
      </div>
    </td>
  </tr>
</table>

<table width="100%" style="margin-bottom: 24px;">
  <tr>
    <td style="padding: 0 8px; font-size: 12px;" width="50%">
      <div style="margin-bottom: 4px;"><strong>Date:</strong> {}</div>
      <div style="margin-bottom: 4px;"><strong>Due:</strong> {}</div>
      <div><strong>Terms:</strong> {}</div>
    </td>
    <td style="padding: 0 8px; font-size: 12px;" width="50%"></td>
  </tr>
</table>

<table width="100%" style="margin-bottom: 16px; border-collapse: collapse;">
  <thead>
    <tr>
      <th style="text-align: left; {th_style}">Description</th>
      <th style="text-align: right; {th_style}">Qty</th>
      <th style="text-align: right; {th_style}">Unit Price</th>
      <th style="text-align: right; {th_style}">Amount</th>
    </tr>
  </thead>
  <tbody>
    {}
  </tbody>
</table>

<table width="100%">
  <tr>
    <td width="50%"></td>
    <td width="50%">
      <table width="100%" style="border-collapse: collapse;">
        <tr><td style="padding: 4px 8px;">Subtotal</td><td style="padding: 4px 8px; text-align: right;">{}</td></tr>
        {}{}{}
        <tr><td style="padding: 8px 8px 4px; font-weight: bold; font-size: 14px; border-top: 2px solid {COLOR_PRIMARY};">Total ({})</td><td style="padding: 8px 8px 4px; text-align: right; font-weight: bold; font-size: 14px; border-top: 2px solid {COLOR_PRIMARY};">{}</td></tr>
      </table>
    </td>
  </tr>
</table>

{}

<table width="100%" style="margin-top: 32px;">
  <tr>
    <td style="border-top: 1px solid {COLOR_BORDER}; padding-top: 8px; font-size: 11px; color: {COLOR_MUTED}; text-align: center;">Thank you for your business</td>
  </tr>
</table>

</body>
</html>"#,
        invoice_number,
        status_color,
        status,
        seller_name,
        seller_address,
        seller_email,
        seller_phone,
        seller_tax_id,
        billing_name,
        billing_address,
        billing_email,
        billing_phone,
        billing_tax_id,
        issue_date,
        due_date,
        payment_terms,
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
