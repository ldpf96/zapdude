/**
 * Zapdude unread badge source.
 *
 * Every account page samples its OWN document title and ships the unread
 * counter it carries: WhatsApp Web publishes "(n) WhatsApp" natively and
 * Messenger does the same (the Facebook bridge's title watcher uses exactly
 * this source). One truthful, self-maintaining number per account - no chat
 * list selectors to drift with WhatsApp/Facebook redesigns. If a future
 * title format drops the "(n)", the count becomes 0 and the badge simply
 * stays hidden (degrade, never fake data).
 *
 * Ships only on CHANGE (3s pacing, page-side timer): the event ring never
 * floods. Delivery goes through the standard beacon transport (the host
 * tags every event with the account + service).
 */
(function () {
    'use strict';

    if (window.__zapdudeUnreadV1) {
        return;
    }
    window.__zapdudeUnreadV1 = true;

    var last = null;

    function ship(count) {
        try {
            if (window.__zapdudeApiV1 && typeof window.__zapdudeApiV1.pushEvent === 'function') {
                window.__zapdudeApiV1.pushEvent(JSON.stringify({
                    kind: 'unread',
                    count: count,
                }));
            }
        } catch (_shipError) {}
    }

    function sample() {
        try {
            var match = String(document.title || '').match(/\((\d+)\)/);
            var count = match ? parseInt(match[1], 10) : 0;
            if (!isFinite(count) || count < 0) {
                count = 0;
            }
            if (count > 999) {
                count = 999; // a runaway parse never paints "1000"
            }
            if (count !== last) {
                last = count;
                ship(count);
            }
        } catch (_sampleError) {
            // next beat
        }
    }

    function start() {
        sample();
        setInterval(sample, 3000);
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', start);
    } else {
        start();
    }
})();
