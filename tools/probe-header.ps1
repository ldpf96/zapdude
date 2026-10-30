# Probe: what chat is open in an account and which header buttons exist.
param([string]$Account = "account-1")

$cfg = [System.IO.File]::ReadAllText("$env:APPDATA\app.zapzap.desktop\config.json") | ConvertFrom-Json
$headers = @{ "X-API-Key" = $cfg.api.apiKey }

$js = @'
(function () {
    try {
        var h = document.querySelector('[data-testid="conversation-header"]');
        if (!h) {
            return { state: 'no_header' };
        }
        var title = (h.querySelector('header') || h).textContent.replace(/\s+/g, ' ').trim().slice(0, 90);
        var btns = Array.prototype.map.call(h.querySelectorAll('button'), function (b) {
            return {
                aria: String(b.getAttribute('aria-label') || ''),
                tid: String(b.getAttribute('data-testid') || ''),
                text: b.textContent.replace(/\s+/g, ' ').trim().slice(0, 30)
            };
        });
        var anyVoipTestid = !!document.querySelector('[data-testid^="voip"]');
        var voipTestids = Array.prototype.map.call(document.querySelectorAll('[data-testid^="voip"]'), function (n) { return n.getAttribute('data-testid'); });
        var main = document.querySelector('#main');
        var mainTitle = main ? (main.querySelector('header') || main).textContent.replace(/\s+/g, ' ').trim().slice(0, 90) : '';
        return { state: 'ok', headerTitle: title, mainTitle: mainTitle, buttons: btns, anyVoipTestid: anyVoipTestid, voipTestids: voipTestids.slice(0, 6) };
    } catch (e) {
        return { state: 'error', err: String(e && e.message || e) };
    }
})()
'@

$body = @{ account = $Account; script = $js } | ConvertTo-Json -Compress -Depth 4
try {
    $out = Invoke-RestMethod -Uri "http://127.0.0.1:29977/api/_debug/eval" -Method Post -Headers $headers -ContentType "application/json" -Body $body
    $out | ConvertTo-Json -Depth 8
} catch {
    "EVAL_FAIL: $($_.Exception.Message)"
}
