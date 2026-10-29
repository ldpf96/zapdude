(function () {
    var report = [];
    var rows = document.querySelectorAll('#pane-side [data-testid^="list-item"]');
    var count = Math.min(rows.length, 3);
    for (var i = 0; i < count; i++) {
        var row = rows[i];
        var attrs = [];
        Array.prototype.forEach.call(row.attributes, function (attribute) {
            attrs.push({ name: attribute.name, value: String(attribute.value).slice(0, 40) });
        });
        var childAttrs = [];
        var first = row.firstElementChild;
        while (first && childAttrs.length < 12) {
            Array.prototype.forEach.call(first.attributes, function (attribute) {
                if (attribute.name !== 'class' && attribute.name.indexOf('style') === -1) {
                    childAttrs.push({ name: attribute.name, value: String(attribute.value).slice(0, 40) });
                }
            });
            first = first.firstElementChild;
        }
        report.push({ rowAttrs: attrs, childAttrs: childAttrs });
    }
    return { rows: report };
})()
