/**
 * Zapdude call anchor - passive discovery phase.
 *
 * WhatsApp Web's call surface (the emulated popup shown for in-page calls)
 * is draggable, which holds it captive inside the app box: the user wants
 * it either FIXED at its spot or, for the popup variant, freely movable.
 * Before pinning anything down, the exact markup of the call surface must
 * be observed (WhatsApp redesigns it between releases and a guess-based
 * script once broke startup).
 *
 * Phase 1 (this file): MutationObserver that records every voip/call
 * surface element whenever one appears, and ships the structure to the
 * host through the bridge (chat-list beacon transport). Harmless: it never
 * modifies the DOM aliens.
 *
 * Phase 2 (future edits here once the data lands): pin the surface to its
 * default position (drop the drag) and add the popup-close recovery.
 */
(function () {
    'use strict';

    if (window.__zapdudeCallAnchor) {
        return;
    }
    window.__zapdudeCallAnchor = true;

    var last_shipped = 0;
    // One branded notification per call surface NODE: a WeakSet keyed on
    // the element itself - an ongoing-call panel keeps mutating for the
    // whole call and would 2.5s-spam toasts forever without this.
    var notifiedNodes = new WeakSet();

    function isCallSurface(node) {
        try {
            if (node.nodeType !== 1) {
                return false;
            }
            var testid = String(node.getAttribute && node.getAttribute('data-testid') || '');
            if (/voip|call/i.test(testid) && !/log|bubble|icce/i.test(testid)) {
                var rect = node.getBoundingClientRect();
                // 80px gate (was 120): WhatsApp's incoming-call PROMPT is a
                // short horizontal bar - the 120px height gate rejected it
                // (the user's test calls before the 09:13 restart shipped
                // nothing; the ring resets on every server start).
                return rect.width > 80 && rect.height > 80;
            }
            var aria = String((node.getAttribute && node.getAttribute('aria-label')) || '');
            return /current call|llamada actual|llamada en curso|incoming (voice|video) call|llamada entrante/i.test(aria)
                && rect.width > 80 && rect.height > 80;
        } catch (_inspectError) {
            return false;
        }
    }

    /// Best-effort caller identity, ORDERED by stability: the prompt has a
    /// semantic testid carrying the caller's display name
    /// (`voip-call-participant-info-name`, from the real captured markup),
    /// then a human text line, then the @handle fallback (in cross-account
    /// calls the handle can be the CALLEE's own).
    function callerName(node) {
        try {
            var named = node.querySelector('div[data-testid="voip-call-participant-info-name"]');
            if (named) {
                var clean = String(named.textContent || '').replace(/\s+/g, ' ').trim();
                if (clean.length >= 2 && clean.length <= 40) {
                    return clean;
                }
            }
            var text = String(node.textContent || '').replace(/\s+/g, ' ').trim();
            var handle = text.match(/@[\w.\-]{2,32}/);
            var handleName = handle ? handle[0] : '';
            var lines = text.split(/(?:\u2022|\||Incoming voice call|Incoming video call|Voice call|Video call|Llamada entrante de voz|Llamada entrante|Llamadas|llamada)/i);
            for (var i = 0; i < lines.length; i++) {
                var line = lines[i].replace(/^[~\u007e*]+/, '').trim();
                if (!line || line.indexOf('@') === 0) {
                    continue;
                }
                var stripped = line.replace(/^(missed|incoming|llamada perdida)\s*/i, '').trim();
                if (stripped.length >= 2 && stripped.length <= 40) {
                    return stripped;
                }
            }
            return handleName;
        } catch (_nameError) {}
        return '';
    }

    /// The surface carries the caller's avatar as a REAL https CDN url
    /// (from the captured markup!) - it passes STRAIGHT through: the
    /// notifier's icon cache already downloads https sources itself. Blob
    /// URLs (seen on some deployments) decode through FileReader instead.
    function callerAvatarDataUrl(node, callback) {
        try {
            var img = node.querySelector ? node.querySelector('img') : null;
            var src = img ? String(img.getAttribute('src') || '') : '';
            if (!src) {
                callback('');
                return;
            }
            if (src.indexOf('https://') === 0) {
                callback(src.slice(0, 300000));
                return;
            }
            if (src.indexOf('data:') === 0) {
                callback(src.slice(0, 300000));
                return;
            }
            if (src.indexOf('blob:') === 0) {
                fetch(src).then(function (response) {
                    return response.blob();
                }).then(function (blob) {
                    var reader = new FileReader();
                    reader.onload = function () {
                        try {
                            var url = String(reader.result || '');
                            callback(url.length <= 300000 ? url : '');
                        } catch (_readError) {
                            callback('');
                        }
                    };
                    reader.onerror = function () { callback(''); };
                    reader.readAsDataURL(blob);
                }).catch(function () { callback(''); });
                return;
            }
            callback('');
        } catch (_avatarError) {
            callback('');
        }
    }

    /// The UNIFORM notification: same payload shape the message
    /// notifications ship, so notifier.rs renders the exact same branded
    /// toast (round avatar, bold contact, one grey line). The in-page
    /// prompt stays functional; this is the visibility layer (a parked
    /// account would otherwise ring invisibly).
    function shipCallNotification(node, mode) {
        try {
            // One toast per surface node (the ongoing panel mutates all the
            // time; WeakSet identity stops the repeats).
            if (notifiedNodes.has(node)) {
                return;
            }
            notifiedNodes.add(node);
            // Titles reach the toast clean: the participant-info testid
            // carries WhatsApp's "~" presence glyph in its text (real
            // capture shipped "~Luis D. Pastor" once) - strip it here.
            var name = callerName(node);
            var tidy = String(name || '').replace(/\s+/g, ' ')
                .replace(/^[~\u007e\*\u2022\ufeff]+/, '').trim();
            // TheMis call Notification WhatsApp builds LATER carries the
            // bare HANDLE as its title (real capture: '@ldpfpe') while
            // this ring surface just taught us the caller's display
            // name. Stash handle -> name so the hook can label the
            // missed toast with the human identity too.
            try {
                var map = window.__zapdudeCallerMap || (window.__zapdudeCallerMap = {});
                var handle = String(node.textContent || '').match(/@[A-Za-z0-9._\-]{3,32}/);
                if (tidy && handle && tidy.indexOf('@') !== 0) {
                    // textContent fuses sibling nodes ("@ldpfpeVoice call")
                    // - crop the fused call word off the key.
                    var key = handle[0].replace(/(Voice|Video|Call|Llamada|Llamando|Entrante).*$/i, '');
                    map[key] = tidy;
                }
            } catch (_mapError) {}
            var payload = {
                kind: 'notification',
                title: tidy || 'Incoming call',
                body: mode === 'video' ? 'Incoming video call'
                    : mode === 'active' ? 'Ongoing call'
                    : 'Incoming voice call',
                tag: 'zapdude-call-' + String(Date.now()),
                require_interaction: false,
                silent: false,
            };
            callerAvatarDataUrl(node, function (avatar) {
                if (avatar) {
                    payload.icon = avatar;
                }
                try {
                    if (window.__zapdudeApiV1 && typeof window.__zapdudeApiV1.pushEvent === 'function') {
                        window.__zapdudeApiV1.pushEvent(JSON.stringify(payload));
                    }
                } catch (_notifyError) {}
            });
        } catch (_shipError) {}
    }

    function describe(node) {
        var rect = { width: 0, height: 0, left: 0, top: 0 };
        var aria = '';
        var testid = '';
        var cls = '';
        var buttons = [];
        var transform = '';
        var outerHtml = '';
        try {
            var bounds = node.getBoundingClientRect();
            rect = {
                left: Math.round(bounds.left),
                top: Math.round(bounds.top),
                width: Math.round(bounds.width),
                height: Math.round(bounds.height),
            };
            aria = String(node.getAttribute('aria-label') || '');
            testid = String(node.getAttribute('data-testid') || '');
            cls = String(node.className || '').slice(0, 90);
            transform = String(node.style.transform || '');
            node.querySelectorAll('button, [role="button"]').forEach(function (button) {
                var label = String(button.getAttribute('aria-label') || '');
                var icon = '';
                var iconNode = button.querySelector('span[data-icon]');
                if (iconNode) {
                    icon = String(iconNode.getAttribute('data-icon') || '');
                }
                if (label || icon) {
                    buttons.push({ label: label.slice(0, 40), icon: icon });
                }
            });
            // Phase 2 fuel: the RAW markup so a real outgoing call gives
            // the exact classes/testids for brand styling the call UI
            // (the user flagged the in-page call prompt as "cuadradas"). 
            outerHtml = String(node.outerHTML || '').slice(0, 6000);
        } catch (_describeError) {}
        return {
            rect: rect,
            aria: aria.slice(0, 60),
            testid: testid,
            cls: cls,
            transform: transform,
            buttons: buttons.slice(0, 12),
            outer_html: outerHtml,
        };
    }

    function isVideoCall(node) {
        try {
            return String(node.textContent || '').toLowerCase().indexOf('video') !== -1
                || /video/i.test(String(node.getAttribute('aria-label') || ''));
        } catch (_videoError) {
            return false;
        }
    }

    function isActiveCall(node) {
        try {
            var composed = (String(node.textContent || '') + ' ' + String(node.getAttribute('aria-label') || '')).toLowerCase();
            return /current call|llamada actual|llamada en curso|rejoin|volviendo/.test(composed)
                && !/accept|decline|incoming/i.test(composed);
        } catch (_activeError) {
            return false;
        }
    }

    /// Outgoing ring surfaces (the caller's own "Llamando…" / "Calling…"
    /// bar, testids starting `voip_outgoing_`) carry no caller identity and
    /// no news for a parked account: the caller is already IN the call.
    /// Shipping a branded toast for that surface produced the bogus
    /// "Llamando… + Incoming voice call" notification (real capture,
    /// 2026-10-30 09:41:47, on the initiating account). Incoming prompts
    /// never start with these words in either language - verified against
    /// the actual ring text "~Luis D. Pastor | @ldpfpe | Voice call".
    function isOutgoingRing(node) {
        try {
            var tid = String(node.getAttribute('data-testid') || '');
            if (tid.indexOf('voip_outgoing_') === 0) {
                return true;
            }
            var text = String(node.textContent || '').replace(/\s+/g, ' ').trim();
            return /^(llamando|calling|ringing|marcando|sonando)\b/i.test(text);
        } catch (_outgoingError) {
            return false;
        }
    }

    function scan() {
        try {
            var findings = [];
            var candidates = document.querySelectorAll('[data-testid], [aria-label]');
            for (var index = 0; index < candidates.length; index++) {
                var node = candidates[index];
                if (!isCallSurface(node)) {
                    continue;
                }
                if (node.contains && Array.prototype.some.call(findings, function (found) {
                    return found.contains(node);
                })) {
                    continue; // only the outermost surface ships
                }
                if (isOutgoingRing(node)) {
                    continue; // the caller side of a call is not news
                }
                findings.push(node);
            }
            if (!findings.length) {
                return;
            }
            // The branded visibility layer: same shape as message
            // notifications (the user asked for the exact same look).
            // Ring/answer surfaces only - an ongoing-call panel is noise
            // (the user is already in that call).
            //
            // BEACON SLOT LAW (learned live: the capture overwrote the
            // call notification in the single-hash slot within one poll
            // beat - the user's test calls shipped zero call toasts): when
            // a notification is going to ship, this scan DOES NOT ship the
            // capture payload; the next mutation beat captures instead and
            // the notification keeps its slot.
            var notifiable = false;
            (function () {
                try {
                    var surface = findings[0];
                    var mode = isActiveCall(surface) ? 'active'
                        : (isVideoCall(surface) ? 'video' : 'voice');
                    if (mode !== 'active') {
                        notifiable = !notifiedNodes.has(surface);
                    }
                } catch (_modeError) {}
            })();
            if (notifiable) {
                try {
                    shipCallNotification(findings[0], (function () {
                        var surface = findings[0];
                        return isVideoCall(surface) ? 'video' : 'voice';
                    })());
                } catch (_notifyError) {}
                return; // slot reserved for the notification
            }
            if (Date.now() - last_shipped < 4000) {
                return; // debounce repeated captures of the same surface
            }
            last_shipped = Date.now();
            var surfaces = findings.map(describe);
            var payload = {
                kind: 'call_surface',
                booted: true,
                surfaces: surfaces,
            };
            try {
                if (window.__zapdudeApiV1 && typeof window.__zapdudeApiV1.pushEvent === 'function') {
                    window.__zapdudeApiV1.pushEvent(JSON.stringify(payload));
                }
            } catch (_bridgeError) {}
        } catch (_scanError) {}
    }

    var scheduled = null;
    function scheduleScan() {
        if (scheduled) {
            return;
        }
        scheduled = setTimeout(function () {
            scheduled = null;
            scan();
        }, 350);
    }

    function start() {
        scan();
        try {
            var observer = new MutationObserver(scheduleScan);
            observer.observe(document.documentElement, {
                childList: true,
                subtree: true,
                attributes: false,
            });
        } catch (_observerError) {}
    }

    if (document.readyState === 'loading') {
        document.addEventListener('DOMContentLoaded', start);
    } else {
        start();
    }
})();
