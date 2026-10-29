(function () {
    return {
        paneSide: !!document.querySelector('#pane-side'),
        listItems: document.querySelectorAll('div[role="listitem"]').length,
        listItemsPane: document.querySelectorAll('#pane-side div[role="listitem"]').length,
        genericRoleRow: document.querySelectorAll('[role="row"]').length,
        genericRoleGrid: document.querySelectorAll('[role="grid"]').length,
        ariaChatList: !!document.querySelector('[aria-label*="Chat"]'),
        divCounts: [
            ['#app > div', document.querySelectorAll('#app > div').length],
            ['div[style]', document.querySelectorAll('div[style]').length],
        ],
        composerTextboxes: document.querySelectorAll('div[role="textbox"]').length,
    };
})()
