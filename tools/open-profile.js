(function () {
    var button = document.querySelector('button[aria-label="You"]');
    if (!button) {
        return { clicked: false, reason: 'You button not found' };
    }
    button.click();
    return { clicked: true };
})()
