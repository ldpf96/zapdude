(function () {
    var button = document.querySelector('button[aria-label="Close"]');
    if (button) {
        button.click();
        return { closed: true };
    }
    return { closed: false, alreadyClean: true };
})()
