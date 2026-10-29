$listener = [System.Net.HttpListener]::new()
$listener.Prefixes.Add("http://127.0.0.1:39990/")
$listener.Start()
$captureFile = "C:\Users\PC\Desktop\Projects\Zapdude\zapdude-rs\webhook-capture.jsonl"
[System.IO.File]::WriteAllText($captureFile, "")
for ($i = 0; $i -lt 4; $i++) {
    try {
        $ctx = $listener.GetContext()
    } catch { break }
    $req = $ctx.Request
    $body = (New-Object System.IO.StreamReader($req.InputStream, $req.ContentEncoding)).ReadToEnd()
    [System.IO.File]::AppendAllText($captureFile, (
        @{
            path  = $req.Url.PathAndQuery
            sig   = $req.Headers["X-Zapdude-Signature"]
            ctype = $req.ContentType
            body  = $body
        } | ConvertTo-Json -Compress
    ) + "`n")
    $resp = $ctx.Response
    $resp.StatusCode = 200
    $bytes = [System.Text.Encoding]::UTF8.GetBytes('{"received":true}')
    $resp.OutputStream.Write($bytes, 0, $bytes.Length)
    $resp.Close()
}
$listener.Stop()
