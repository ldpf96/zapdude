(function () {
    var hits = [];
    try {
        for (var index = 0; index < localStorage.length; index++) {
            var key = String(localStorage.key(index));
            var value = String(localStorage.getItem(key) || '');
            if (value.length > 4000) { value = value.slice(0, 4000); }
            var numberMatch = value.match(/\+?1[\d ()-]{9,}/);
            if (numberMatch) {
                hits.push({ key: key, extract: numberMatch[0].slice(0, 24) });
            }
        }
    } catch (error) {
        return { ok: false, error: String((error && error.message) || error) };
    }
    return {
        ok: true,
        keyCount: localStorage.length,
        phoneKeys: hits.slice(0, 10),
    };
})()
