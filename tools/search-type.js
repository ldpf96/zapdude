(function () {
    function setNativeValue(input, value) {
        var setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
        setter.call(input, value);
        input.dispatchEvent(new Event('input', { bubbles: true }));
    }
    var input = document.querySelector('[data-testid="chatlist-search"] input')
        || document.querySelector('input[aria-label*="search" i], input[aria-label*="Buscar"], input[aria-label*="Busca"], input[title*="search" i]');
    if (!input) {
        return { ok: false, error: 'search_input_not_found' };
    }
    input.focus({ preventScroll: true });
    setNativeValue(input, '51987931332');
    return { ok: true, typed: true };
})()
