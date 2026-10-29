(function () {
    var rows = document.querySelectorAll('#pane-side [data-testid^="list-item"]');
    var report = { rowCount: rows.length, rows: [] };
    var limit = Math.min(rows.length, 3);
    for (var i = 0; i < limit; i++) {
        var row = rows[i];
        var testids = [];
        row.querySelectorAll('[data-testid]').forEach(function (node) {
            var id = node.getAttribute('data-testid');
            if (id && id !== row.getAttribute('data-testid')) {
                testids.push(id);
            }
        });
        var aria = [];
        row.querySelectorAll('[aria-label]').forEach(function (node) {
            aria.push(String(node.getAttribute('aria-label') || '').slice(0, 60));
        });
        var titles = [];
        row.querySelectorAll('span[title]').forEach(function (span) {
            titles.push(span.getAttribute('title'));
        });
        var iconNames = [];
        row.querySelectorAll('span[data-icon]').forEach(function (span) {
            iconNames.push(span.getAttribute('data-icon'));
        });
        report.rows.push({
            testid: row.getAttribute('data-testid'),
            innerTestids: testids.slice(0, 10),
            ariaLabels: aria.slice(0, 8),
            spanTitles: titles.slice(0, 8),
            icons: iconNames.slice(0, 8),
        });
    }
    return report;
})()
