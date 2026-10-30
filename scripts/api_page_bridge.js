/**
 * Zapdude local API page bridge.
 *
 * Exposes a tiny, defensive read-and-act surface over the WhatsApp Web DOM
 * for the local automation API. Every entry point returns a plain JSON-able
 * object and never throws: the automation layer must not take the embedded
 * page down when WhatsApp changes its markup.
 *
 * Injected as a resident main-world initialization script by the API module.
 *
 * Response transport (in order of preference):
 *   1. window.__zapdude.respond(serializedJson) - native bridge object when
 *      the host exposes one (WebView2 host-injected helper).
 *   2. location hash beacon (`#__zapdude=<urlencoded payload>`) polled by the
 *      host through WebviewWindow::url().
 */
(function () {
    'use strict';

    var BRIDGE_ID = '__zapdudeApiV1';

    function emitError(operation, error) {
        return {
            ok: false,
            error: 'unexpected_error',
            detail: String((error && error.message) || error || ''),
            operation: operation,
        };
    }

    function findComposer() {
        const selectors = [
            'footer div[contenteditable="true"]',
            'div[role="textbox"][data-tab="10"]',
            'div[role="textbox"][data-tab="6"]',
            'div[role="textbox"][contenteditable="true"]',
            'div[contenteditable="true"][data-tab]',
        ];
        for (const selector of selectors) {
            const nodes = document.querySelectorAll(selector);
            for (const node of nodes) {
                try {
                    if (node.offsetParent !== null && !node.closest('div[role="dialog"]')) {
                        return node;
                    }
                } catch (_hiddenCheckError) {
                    return node;
                }
            }
        }
        return null;
    }

    function decodeFinalUrl() {
        try {
            return String(location.href || '');
        } catch (_error) {
            return '';
        }
    }

    function visibleChatListItems() {
        // WhatsApp moved the chat list to grid rows with stable test ids
        // (data-testid="list-item-N"); the role-based probing below stays
        // as a fallback for older deployments.
        const hosts = [
            '#pane-side [data-testid^="list-item"]',
            '#pane-side div[role="listitem"]',
            'div[role="listitem"]',
        ];
        let items = [];
        for (const selector of hosts) {
            items = Array.from(document.querySelectorAll(selector));
            if (items.length) break;
        }
        return items;
    }

    function scrapeChatList(limit) {
        try {
            const max = Math.max(1, Math.min(100, Number(limit) || 20));
            const items = visibleChatListItems();
            const chats = [];
            for (const item of items.slice(0, max)) {
                let title = '';
                let unread = false;
                let unreadCount = 0;
                let snippet = '';
                try {
                    const titleNode = item.querySelector('span[title]');
                    title = titleNode ? titleNode.getAttribute('title') : '';
                    // Unread badge: the dedicated test id first, then the
                    // historical aria-label matcher; digits come from the
                    // badge itself so locale words never matter.
                    const badge =
                        item.querySelector('[data-testid="icon-unread-count"]')
                        || item.querySelector('[aria-label*="read"], [aria-label*="lidos"], [aria-label*="no ley"]');
                    if (badge) {
                        const labelCount = String(badge.getAttribute('aria-label') || '').match(/(\d+)/);
                        unreadCount = labelCount ? Number(labelCount[1]) : 1;
                        unread = true;
                    }
                    if (!unread && item.querySelector('span[aria-label*="missed"]')) {
                        unread = true;
                    }
                    // Last-message preview: the secondary cell's titled span,
                    // falling back to the last titled span in the row.
                    const secondary =
                        item.querySelector('[data-testid="cell-frame-secondary"] span[title]');
                    snippet = secondary
                        ? String(secondary.getAttribute('title') || '')
                        : (function () {
                            const titled = item.querySelectorAll('span[title]');
                            return titled.length
                                ? String(titled[titled.length - 1].getAttribute('title') || '')
                                : '';
                        })();
                    if (snippet.length > 120) snippet = snippet.slice(0, 120);
                } catch (_itemParseError) {
                    // Skip malformed items; the list remains best-effort.
                }
                chats.push({
                    title: title,
                    unread: unread,
                    unread_count: unreadCount,
                    snippet: snippet,
                });
            }
            return { ok: true, count: chats.length, chats: chats, degraded: chats.length === 0 };
        } catch (error) {
            return emitError('chatList', error);
        }
    }

    function pageScript() {
        const api = {
            version: 1,

            /** Report the composer state without touching the page. */
            composerInfo: function () {
                try {
                    const composer = findComposer();
                    if (!composer) {
                        return {
                            ok: true,
                            ready: false,
                            reason: 'not_ready',
                            chat_open: document.querySelector('[aria-label="Chat list"], #pane-side, div[role="listitem"]') != null,
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
                    return emitError('composerInfo', error);
                }
            },

            /** Outline page level state for status/debug endpoints. */
            pageInfo: function () {
                try {
                    return {
                        ok: true,
                        url: decodeFinalUrl(),
                        title: String(document.title || ''),
                        chat_list: !!document.querySelector('[aria-label="Chat list"], #pane-side, div[role="listitem"]'),
                        composer: !!findComposer(),
                        qrcode: document.querySelectorAll('canvas').length > 0,
                    };
                } catch (error) {
                    return emitError('pageInfo', error);
                }
            },

            /**
             * Insert text into the open composer and dispatch Enter.
             * Uses a resilient paste-as-plain-text strategy (Lexical-aware).
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
                    function normalizeForCompare(value) {
                        return String(value || '')
                            .replace(/[\u200b-\u200d\u200e\u200f\ufeff]/g, '')
                            .replace(/\s+/g, ' ')
                            .trim();
                    }
                    let inserted = false;
                    let skippedInsert = false;
                    const already = normalizeForCompare(composer.innerText);
                    const wanted = normalizeForCompare(text);
                    if (wanted && (already === wanted || already.endsWith(wanted))) {
                        // WhatsApp prefilled the composer from the URL; the
                        // message is already typed, only the send press remains.
                        skippedInsert = true;
                    }

                    // Controlled Lexical editors prefer a synthetic paste.
                    if (!skippedInsert && composer.hasAttribute('data-lexical-editor')) {
                        try {
                            const transfer = new DataTransfer();
                            transfer.setData('text/plain', text);
                            const pasteEvent = new ClipboardEvent('paste', {
                                clipboardData: transfer,
                                bubbles: true,
                                cancelable: true,
                            });
                            composer.dispatchEvent(pasteEvent);
                            inserted = pasteEvent.defaultPrevented;
                        } catch (_lexPasteError) {
                            inserted = false;
                        }
                    }

                    if (!inserted) {
                        try {
                            if (text.indexOf('\n') !== -1) {
                                const lines = text.split('\n');
                                let wrote = false;
                                for (const [index, line] of lines.entries()) {
                                    if (index > 0) {
                                        if (!document.execCommand('insertLineBreak', false, null)) break;
                                    }
                                    if (line) {
                                        if (!document.execCommand('insertText', false, line)) break;
                                    }
                                    wrote = true;
                                }
                                inserted = wrote;
                            } else {
                                inserted = document.execCommand('insertText', false, text);
                            }
                        } catch (_execCommandError) {
                            inserted = false;
                        }
                    }

                    let pressed = true;
                    try {
                        const base = {
                            key: 'Enter',
                            code: 'Enter',
                            keyCode: 13,
                            which: 13,
                            bubbles: true,
                            cancelable: true,
                            composed: true,
                        };
                        composer.dispatchEvent(new KeyboardEvent('keydown', base));
                        composer.dispatchEvent(new KeyboardEvent('keypress', base));
                        composer.dispatchEvent(new KeyboardEvent('keyup', base));
                    } catch (_enterError) {
                        pressed = false;
                    }

                    if (!pressed) {
                        // Last resort: click the visible send button.
                        try {
                            const buttons = document.querySelectorAll(
                                'button[data-tab="send"], span[data-icon="send"], button[aria-label]'
                            );
                            for (const button of buttons) {
                                if (String(button.getAttribute('aria-label') || '').includes('end')
                                    || !!button.querySelector('span[data-icon="send"]')) {
                                    button.click();
                                    pressed = true;
                                    break;
                                }
                            }
                        } catch (_clickFallbackError) {
                            pressed = false;
                        }
                    }

                    return {
                        ok: true,
                        inserted: inserted,
                        skipped_insert: skippedInsert,
                        pressed: pressed,
                        text_length: text.length,
                    };
                } catch (error) {
                    return emitError('sendCurrent', error);
                }
            },

            /** Best-effort scrape of the visible chat list. */
            chatList: function (limit) {
                return scrapeChatList(limit);
            },

            /**
             * Open a chat (and optionally prefill/send a message) through
             * the search drawer instead of the ?send URL deep link - the
             * deep link traps cold-started webviews on an unresolvable
             * redirect (about:blank, seen live). The drawer works from any
             * state and matches unsaved numbers; the host paces the steps
             * (drawer mounts asynchronously), so this op runs in pieces:
             * first call opens the drawer, later calls type and pick.
             */
            openChatByPhone: function (phone) {
                try {
                    phone = String(phone || '').replace(/\D+/g, '');
                    if (phone.length < 5) {
                        return { ok: false, error: 'phone_too_short' };
                    }
                    var drawer = null;
                    try {
                        drawer = document.querySelector('div[data-testid="new-chat-drawer"]');
                    } catch (_queryError) {
                        drawer = null;
                    }
                    if (!drawer) {
                        var opener = document.querySelector(
                            'button[aria-label="New chat"], button[aria-label*="nuevo chat" i], ' +
                            'button[aria-label*="chat nuevo" i], button[aria-label*="new chat" i]'
                        );
                        if (!opener) {
                            return { ok: false, error: 'new_chat_button_not_found' };
                        }
                        opener.click();
                        return { ok: false, stage: 'drawer_opening', error: 'retry_needed' };
                    }
                    var input = drawer.querySelector('input');
                    if (!input) {
                        return { ok: false, error: 'drawer_input_not_found' };
                    }
                    input.focus({ preventScroll: true });
                    var setter = Object.getOwnPropertyDescriptor(
                        window.HTMLInputElement.prototype,
                        'value'
                    ).set;
                    setter.call(input, '+' + phone);
                    input.dispatchEvent(new Event('input', { bubbles: true }));
                    return { ok: true, typed: true };
                } catch (error) {
                    return emitError('openChatByPhone', error);
                }
            },

            /**
             * Pick the drawer's result row that matches the searched number
             * (runs one beat after openChatByPhone's typing, when the
             * drawer rendered matches).
             */
            pickPhoneResult: function (phone) {
                try {
                    phone = String(phone || '').replace(/\D+/g, '');
                    var drawer = document.querySelector('div[data-testid="new-chat-drawer"]');
                    if (!drawer) {
                        return { ok: false, error: 'drawer_gone', stage: 'pick' };
                    }
                    var rows = drawer.querySelectorAll('div[data-testid="cell-frame-container"]');
                    var row = null;
                    // Own-number search: WhatsApp renders the self-chat row.
                    try {
                        row = drawer.querySelector('div[data-testid="message-yourself-row"]');
                    } catch (_selfRowError) {
                        row = null;
                    }
                    if (!row) {
                        for (var index = 0; index < rows.length; index++) {
                            var text = String(rows[index].textContent || '');
                            if (/matches this number|coincide con este n/i.test(text)) {
                                row = rows[index];
                                break;
                            }
                        }
                    }
                    if (!row && rows.length === 1) {
                        row = rows[0];
                    }
                    if (!row) {
                        return { ok: false, error: 'no_matching_row', stage: 'pick' };
                    }
                    var target = row;
                    var depth = 0;
                    while (target && target !== drawer && target.getAttribute('role') !== 'row' && depth < 8) {
                        target = target.parentElement;
                        depth += 1;
                    }
                    (target && target !== drawer ? target : row).click();
                    return { ok: true, clicked: true };
                } catch (error) {
                    return emitError('pickPhoneResult', error);
                }
            },

            /**
             * Start a call from the OPEN chat: clicks the voice/video call
             * button in the conversation header. Locale-aware matching so
             * Spanish/English deployments both work. Returns diagnostics on
             * failure (automation flows surface them to the API caller).
             */
            startCall: function (mode) {
                try {
                    var header = document.querySelector('[data-testid="conversation-header"]');
                    if (!header) {
                        return { ok: false, error: 'no_open_chat' };
                    }
                    var wantVideo = String(mode || 'voice') === 'video';
                    var buttons = header.querySelectorAll('button');
                    // Locale-aware, TWO-PASS so markup drift cannot strand the
                    // endpoint (real live capture, 2026-10-30: this deployment
                    // labels the voice button just "Llamada" - the older
                    // "Llamada de voz" / "Voice call" wordings still get the
                    // strict pass first; the strict list never matches the
                    // video button even in voice mode).
                    var isVideoLabel = function (label) {
                        return /videollamada|video\s*call/i.test(label);
                    };
                    var isVoiceStrict = function (label) {
                        return /^llamada(\s+de\s+(voz|audio))?$|^llamar\b|^voice\s*call\b|^llamada\s*de\s*voz/i.test(label);
                    };
                    var isVoiceLenient = function (label) {
                        return /llamad|^llamar|call/i.test(label) && !/video/i.test(label);
                    };
                    // First pass: exact label matches only.
                    var pass = wantVideo ? isVideoLabel : isVoiceStrict;
                    for (var index = 0; index < buttons.length; index++) {
                        var label = String(buttons[index].getAttribute('aria-label') || '').trim();
                        if (label && pass(label)) {
                            buttons[index].click();
                            return { ok: true, clicked: label.slice(0, 32) };
                        }
                    }
                    // Second pass (voice only): tolerate drift by taking the
                    // first call-ish label that is definitely not the video
                    // button.
                    if (!wantVideo) {
                        for (var second = 0; second < buttons.length; second++) {
                            var alt = String(buttons[second].getAttribute('aria-label') || '').trim();
                            if (alt && isVoiceLenient(alt)) {
                                buttons[second].click();
                                return { ok: true, clicked: alt.slice(0, 32), degraded: true };
                            }
                        }
                    }
                    var labels = Array.prototype.map.call(buttons, function (b) {
                        return String(b.getAttribute('aria-label') || '');
                    }).filter(Boolean);
                    return { ok: false, error: 'call_button_not_found', labels: labels.slice(0, 6) };
                } catch (error) {
                    return emitError('startCall', error);
                }
            },

            /**
             * End the active call: looks for the hang-up control in the call
             * surface (covers the in-page call panel and whatever the call
             * popup renders). Non-fatal when no call exists.
             */
            endCall: function () {
                try {
                    var pattern = /colgar|end\s*call|finalizar\s*llamada/i;
                    var buttons = document.querySelectorAll('button');
                    for (var index = 0; index < buttons.length; index++) {
                        var label = String(buttons[index].getAttribute('aria-label') || '');
                        if (pattern.test(label)) {
                            buttons[index].click();
                            return { ok: true, clicked: label.slice(0, 32) };
                        }
                    }
                    return { ok: false, error: 'no_call_surface' };
                } catch (error) {
                    return emitError('endCall', error);
                }
            },

            /**
             * Reconcile the page after its call popup vanished without
             * going through WhatsApp's own end-call flow (the popup X
             * button bypasses it, the call keeps ringing/burning on the
             * page side, and the "Llamada / Llamando..." pill stays on
             * screen with nothing reachable - the stuck state seen live).
             *
             * Phase 1: click the leftover pill (WhatsApp then re-issues
             * its own call window/shutdown path). Phase 2 (probe mode):
             * report whether a pill would still match, without clicking -
             * the host uses it to decide a page reload as the failsafe.
             */
            callRecover: function (probeOnly) {
                try {
                    var pill_pattern = /\b(\d+\s*min|llamando|calling|reanudar|volviendo|volver a la llamada|return to call|rejoin|llamada en curso|llamada actual|current call|rejoin call)/i;
                    var candidates = document.querySelectorAll(
                        'div[role="button"], button, [role="button"]'
                    );
                    var probe = [];
                    for (var index = 0; index < candidates.length; index++) {
                        var node = candidates[index];
                        try {
                            var label = String(node.getAttribute('aria-label') || '');
                            var text = String(node.textContent || '').replace(/\s+/g, ' ').trim();
                            var composed = (label + ' ' + text).trim();
                            if (!pill_pattern.test(composed)) {
                                continue;
                            }
                            var rect = node.getBoundingClientRect();
                            if (rect.width < 40 || rect.height < 30) {
                                continue;
                            }
                            probe.push(composed.slice(0, 60));
                            if (probeOnly) {
                                return { ok: true, in_call: true, probe: probe };
                            }
                            node.click();
                            return { ok: true, recovered: 'clicked', probe: probe.slice(0, 6) };
                        } catch (_candidateError) {}
                    }
                    return { ok: true, in_call: false, probe: probe.slice(0, 6) };
                } catch (error) {
                    return emitError('callRecover', error);
                }
            },

            /**
             * Dispatch an incoming message event to the host.
             * Runs from the Notification hook and chat list observers.
             */
            pushEvent: function (eventJson) {
                var event = null;
                try {
                    event = JSON.parse(eventJson);
                } catch (_error) {
                    event = { kind: 'unexpected' };
                }
                emitResponse('evt_' + String(Date.now()) + '_' + Math.floor(Math.random() * 1e6), event);
                return { ok: true };
            },
        };

        function hashBeacon(payload) {
            try {
                location.replace('#__zapdude=' + encodeURIComponent(payload));
                return true;
            } catch (_beaconError) {
                return false;
            }
        }

        function emitResponse(id, payload) {
            var envelope = JSON.stringify({ id: String(id), payload: payload });
            try {
                // WebView2 host helpers may be exposed via AddHostObject once
                // a later version enables them; prefer direct call first.
                if (window.__zapdude && typeof window.__zapdude.respond === 'function') {
                    window.__zapdude.respond(envelope);
                    return;
                }
            } catch (_respondError) {
                /* fall through to beacon */
            }
            hashBeacon(envelope);
        }

        function dispatchCommand(id, op, argsJson) {
            var args = {};
            try {
                args = JSON.parse(argsJson || '{}') || {};
            } catch (_parseError) {
                args = {};
            }
            var payload;
            try {
                if (op === 'composerInfo') {
                    payload = api.composerInfo();
                } else if (op === 'pageInfo') {
                    payload = api.pageInfo();
                } else if (op === 'sendCurrent') {
                    payload = api.sendCurrent(String(args.text || ''));
                } else if (op === 'chatList') {
                    payload = api.chatList(Number(args.limit) || 20);
                } else if (op === 'openChatByPhone') {
                    payload = api.openChatByPhone(String(args.phone || ''));
                } else if (op === 'pickPhoneResult') {
                    payload = api.pickPhoneResult(String(args.phone || ''));
                } else if (op === 'startCall') {
                    payload = api.startCall(String(args.mode || 'voice'));
                } else if (op === 'endCall') {
                    payload = api.endCall();
                } else if (op === 'callRecover') {
                    payload = api.callRecover();
                } else {
                    payload = { ok: false, error: 'unknown_operation', operation: String(op || '') };
                }
            } catch (error) {
                payload = emitError(String(op || 'unknown'), error);
            }
            emitResponse(id, payload);
            return { ok: true };
        }

        window.__zapdudeApiV1 = api;
        api.dispatch = function (id, op, argsJson) {
            try {
                return dispatchCommand(id, op, argsJson);
            } catch (error) {
                emitResponse(id, emitError(String(op || 'unknown'), error));
                return { ok: false };
            }
        };

        // ------------------------------------------------------------------
        // Incoming event sources - DISABLED for now.
        //
        // Empirically (WebView2 child webviews), hooking window.Notification
        // or installing DOM observers at boot kills WhatsApp Web's own JS
        // startup: the page stays at its bare dark background (looks "gray")
        // while our bridge keeps working. The local API remains fully
        // usable (status/chats/send via dispatch); push-style events will
        // return behind a safe mechanism later.
        // ------------------------------------------------------------------
        void (0);
    }

    try {
        pageScript();
    } catch (_setupError) {
        // A bridge failure must never disturb the embedded page.
    }
})();
