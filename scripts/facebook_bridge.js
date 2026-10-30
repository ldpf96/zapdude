/**
 * Zapdude Facebook page bridge (adaptive, never-breaking).
 *
 * Lives on the messenger.com document of a "facebook" service account and
 * exposes the SAME `window.__zapdudeApiV1` contract the WhatsApp bridge
 * uses. That is the whole integration model: Zapdude does NOT replace the
 * chat client - the real page loads inside our webview (own session, email
 * + password + 2FA), and this bridge drives/reads the page the way a hand
 * would, adapting to whatever it actually renders.
 *
 * HARD RULES (the "local API never breaks" doctrine):
 *   1. Every entry point catches everything and returns a plain
 *      JSON-able object. A moved Messenger markup degrades into structured
 *      `degraded` fields - it never throws, never fakes success, never
 *      takes the page or the API endpoint down.
 *   2. Selector strategies are LISTS, tried in order; nothing is assumed.
 *      When nothing matches the read is honest: degraded state reported,
 *      no preflight guesses. Future re-anchoring means adding a selector,
 *      not redesigning.
 *   3. Nothing here calls WhatsApp-specific ops. Each service keeps its
 *      own bridge; unknown op names answer `unknown_operation`.
 *
 * Ops (v1, receive-first; send is intentionally not promised until the
 * selectors have been proven live on a real login):
 *   version()       -> contract version
 *   pageInfo()      -> url / title / surface detection (threads, composer,
 *                      login form) - the host derives session_state from it
 *   chatList(limit) -> best-effort visible_threads scrape (multi-strategy,
 *                      unconvinced rows report degraded fields)
 *   composerInfo()  -> open-thread composer presence/length
 *   sendCurrent(t)  -> generic composer write (insertText + Enter), with
 *                      explicit not_ready results instead of blind acting
 *   titleBadge()    -> "(N)" unread counter parsed from document.title
 *
 * Installed DEFERRED (beacon poller keeps it re-installed idempotently);
 * a title-badge watcher (2 s, fired only on CHANGE) ships unread-counter
 * changes to the host as `title_badge` events - observability for agents,
 * NOT toast spam.
 */
(function () {
    'use strict';

    if (window.__zapdudeApiV1) {
        return;
    }

    // ------------------------------------------------------------------
    // Transport to the host: same contract as the WhatsApp bridge. The
    // notification hook may not be installed yet when we start, so both
    // paths must work: direct pushEvent (via the hook's original channel
    // pull is not possible - the SHIPs beacon), else raw beacon.
    // ------------------------------------------------------------------
    function hashBeacon(envelope) {
        try {
            location.replace('#__zapdude=' + encodeURIComponent(envelope));
            return true;
        } catch (_beaconError) {
            return false;
        }
    }

    function ship(id, payload) {
        try {
            hashBeacon(JSON.stringify({ id: String(id), payload: payload }));
            return true;
        } catch (_shipError) {
            return false;
        }
    }

    function emitEvent(payload) {
        return ship(
            'evt_' + String(Date.now()) + '_' + Math.floor(Math.random() * 1e6),
            payload
        );
    }

    // The notification hook (shared with WhatsApp) exposes pushEvent on the
    // api object; when present we use it (stable ordering with hook events)
    // and fall back to the raw beacon otherwise.
    function pageApi() {
        var api = {
            version: 1,
            service: 'facebook',

            /** What the page actually renders right now. */
            pageInfo: function () {
                try {
                    return {
                        ok: true,
                        service: 'facebook',
                        url: String(location.href || ''),
                        title: String(document.title || ''),
                        // Threads list: the left conversations column.
                        chat_list: visibleThreadRows(1).length > 0,
                        // Open thread composer: Facebook renders the write
                        // box as contenteditable inside the conversation.
                        composer: !!findComposer(),
                        // Login surface detection: a password field is the
                        // honest signal (Facebook account login + 2FA).
                        login_form: document.querySelector(
                            'input[type="password"]'
                        ) != null,
                        // Legacy field: the WhatsApp QR concept does not
                        // exist here; always false (status flows keep shape).
                        qrcode: false,
                    };
                } catch (error) {
                    return fail('pageInfo', error);
                }
            },

            /** Composer state for host readiness loops. */
            composerInfo: function () {
                try {
                    const composer = findComposer();
                    if (!composer) {
                        return {
                            ok: true,
                            ready: false,
                            reason: 'not_ready',
                            chat_open: visibleThreadRows(1).length > 0,
                            title: String(document.title || ''),
                        };
                    }
                    let textLength = 0;
                    try {
                        textLength = String(composer.innerText || '').length;
                    } catch (_innerTextError) {
                        textLength = 0;
                    }
                    return {
                        ok: true,
                        ready: true,
                        empty: textLength === 0,
                        text_length: textLength,
                        title: String(document.title || ''),
                    };
                } catch (error) {
                    return fail('composerInfo', error);
                }
            },

            /**
             * Best-effort scrape of the visible thread rows. Multi-strategy:
             * run the selector candidates in order, take the first hit;
             * per row read title/snippet/unread defensively, and be honest
             * about degraded confidence instead of pretending. EXACT HOST
             * CONTRACT: the op is named `chatList` like the WhatsApp
             * bridge's (page_ops::list_chats evaluates that name).
             */
            chatList: function (limit) {
                try {
                    const max = Math.max(1, Math.min(100, Number(limit) || 20));
                    const rows = visibleThreadRows(max);
                    const chats = [];
                    for (const row of rows) {
                        const read = readThreadRow(row);
                        if (read) {
                            chats.push(read);
                        }
                    }
                    return {
                        ok: true,
                        count: chats.length,
                        chats: chats,
                        degraded: chats.length === 0,
                        title: String(document.title || ''),
                    };
                } catch (error) {
                    return fail('chats', error);
                }
            },

            /**
             * Write text into the open thread composer and press Enter.
             * Phase 1 guarantees: never acts blind. When no composer is
             * found the answer is structured (not_ok), the host gates the
             * send endpoint until selectors are proven on a real login.
             */
            sendCurrent: function (text) {
                try {
                    const composer = findComposer();
                    if (!composer) {
                        return { ok: false, error: 'composer_not_found' };
                    }
                    try {
                        composer.focus({ preventScroll: true });
                    } catch (_focusError) {
                        composer.focus();
                    }
                    let inserted = false;
                    try {
                        inserted = document.execCommand('insertText', false, String(text || ''));
                    } catch (_execError) {
                        inserted = false;
                    }
                    let pressed = false;
                    try {
                        const base = {
                            key: 'Enter', code: 'Enter', keyCode: 13, which: 13,
                            bubbles: true, cancelable: true, composed: true,
                        };
                        composer.dispatchEvent(new KeyboardEvent('keydown', base));
                        composer.dispatchEvent(new KeyboardEvent('keyup', base));
                        pressed = true;
                    } catch (_enterError) {
                        pressed = false;
                    }
                    return {
                        ok: inserted && pressed,
                        inserted: inserted,
                        pressed: pressed,
                        text_length: String(text || '').length,
                    };
                } catch (error) {
                    return fail('sendCurrent', error);
                }
            },

            /** the document.title unread counter ("(3) Messenger | ..."). */
            titleBadge: function () {
                try {
                    const title = String(document.title || '');
                    const match = title.match(/^\((\d+)\)\s/);
                    const badge = match ? Number(match[1]) : 0;
                    return { ok: true, badge: badge, title: title };
                } catch (error) {
                    return fail('titleBadge', error);
                }
            },
        };
        return api;
    }

    function fail(operation, error) {
        return {
            ok: false,
            error: 'unexpected_error',
            detail: String((error && error.message) || error || ''),
            operation: operation,
        };
    }

    // ------------------------------------------------------------------
    // Adaptive selectors. Candidate lists, FIRST match wins, nothing is
    // assumed. They describe what messenger.com has been observed
    // rendering; new Facebook deployments first DEGRADE (honest empty
    // reads) and then re-anchor by adding candidates - the API surface
    // above never changes.
    // ------------------------------------------------------------------
    function visibleThreadRows(limit) {
        const candidates = [
            // Conversation list rows (aria-labeled chat entries).
            'div[role="navigation"] div[role="row"]',
            'div[role="row"][aria-label]',
            'ul[role="list"] li[role="listitem"]',
        ];
        for (const selector of candidates) {
            let items = [];
            try {
                items = Array.from(document.querySelectorAll(selector));
                items = items.filter(function (node) {
                    try {
                        return node.offsetParent !== null;
                    } catch (_hiddenError) {
                        return true;
                    }
                }).slice(0, limit);
            } catch (_selectorError) {
                items = [];
            }
            if (items.length) {
                return items;
            }
        }
        return [];
    }

    function readThreadRow(row) {
        try {
            let title = String(row.getAttribute('aria-label') || '');
            let snippet = '';
            // Named avatar + name candidates inside the row.
            let nameNode = null;
            try {
                // A titled element with an image next to it is the classic
                // row head (avatar + name); textContent is the honest
                // fallback for layouts without tiled spans.
                const heads = row.querySelectorAll('span[title], div[title], h2, h3');
                for (const head of heads) {
                    const candidate = String(head.getAttribute('title') || head.textContent || '').trim();
                    if (candidate) {
                        nameNode = head;
                        break;
                    }
                }
            } catch (_headError) {
                nameNode = null;
            }
            if (nameNode) {
                const named = String(nameNode.getAttribute('title') || '');
                title = named || title;
                snippet = String(nameNode.textContent || '').trim();
            } else {
                snippet = String(row.textContent || '').replace(/\s+/g, ' ').trim();
            }
            if (!title && snippet) {
                title = snippet;
            }
            // Unread: messenger marks unread conversation entries with an
            // unread cue (an accessible label naming it). Without a
            // universally stable hook we report the STRONG signal only
            // (explicit unread wording in an aria-label or test id), else
            // leave it flat and honest (unread false) - never a fake
            // unread count.
            let unread = false;
            let unreadCount = 0;
            try {
                const markers = row.querySelectorAll(
                    '[aria-label*="Unread" i], [aria-label*="No leído" i], [data-testid*="unread" i]'
                );
                unread = markers.length > 0;
                const ariaMarkers = row.querySelectorAll('[aria-label*="Unread" i]');
                if (ariaMarkers.length) {
                    const aria = String(ariaMarkers[0].getAttribute('aria-label') || '');
                    const digits = aria.match(/(\d+)/);
                    unreadCount = digits ? Number(digits[1]) : 1;
                }
            } catch (_unreadError) {
                unread = false;
            }
            // A thread row without a title maps to an honest skip: the
            // scrape yields nothing for it (degraded flag stays false at
            // the list level only if SOMETHING is readable).
            if (!title) {
                return null;
            }
            return {
                title: title.slice(0, 120),
                snippet: snippet.slice(0, 120),
                unread: unread,
                unread_count: unreadCount,
            };
        } catch (_rowError) {
            return null;
        }
    }

    // Composer: the write box of an OPEN conversation. Candidate list
    // ordered most-specific first; visibility-checked; dialogs (thread
    // settings composers inside modals) are skipped like whatsapp does.
    function findComposer() {
        const candidates = [
            'form div[contenteditable="true"][role="textbox"]',
            'div[contenteditable="true"][role="textbox"]',
            'div[contenteditable="true"]',
        ];
        for (const selector of candidates) {
            let nodes = [];
            try {
                nodes = Array.from(document.querySelectorAll(selector));
            } catch (_selectorError) {
                nodes = [];
            }
            for (const node of nodes) {
                try {
                    if (node.offsetParent !== null && !node.closest('div[role="dialog"]:not([aria-label="Chats"])')) {
                        return node;
                    }
                } catch (_hiddenCheckError) {
                    return node;
                }
            }
        }
        return null;
    }

    // ------------------------------------------------------------------
    // Title badge watcher: ships unread-counter CHANGES to the host.
    // 2 s interval (host-paced payloads, page-side timer), fires only on
    // change so bursts never flood the event ring. Delivery is the same
    // beacon envelope the notification hook uses; the host tags it with
    // the account (and service). Observability events, NOT toasts.
    // ------------------------------------------------------------------
    var LAST_BADGE = null;
    function watchTitle() {
        try {
            const title = String(document.title || '');
            const match = title.match(/^\((\d+)\)\s/);
            const badge = match ? Number(match[1]) : 0;
            if (badge !== LAST_BADGE) {
                LAST_BADGE = badge;
                emitEvent({
                    kind: 'title_badge',
                    badge: badge,
                    title: title,
                    service: 'facebook',
                });
            }
        } catch (_watchError) {
            // next beat
        }
    }

    window.__zapdudeApiV1 = pageApi();

    try {
        if (window === window.top && !window.__zapdudeTitleWatchV1) {
            window.__zapdudeTitleWatchV1 = true;
            setInterval(watchTitle, 2000);
        }
    } catch (_watchSetupError) {
        // The watcher is optional; the page bridge functions stay usable.
    }
})();
