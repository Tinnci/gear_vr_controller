# Interaction and code design

The main journey is connect, choose a mode, then start control. Input stays paused
after connection, reconnection, calibration or binding changes. Tuning and
troubleshooting have separate pages. Advanced addresses and sensor details are
collapsed by default. Native controls provide keyboard and accessibility names.
The discovery list has a bounded height. Navigation collapses in a narrow window.
A SelectorBar switches fixed subpages under Tune, Settings and Help. Each parent
remembers its selected subpage. A four-row Grid keeps the title and notice,
subpage selector, scrolling content and save bar separate. Unsaved changes keep
the save bar visible across all main pages. Changing routes recreates only the
scrolling content; it does not restart transport, calibration or input testing.

## Responsibility boundaries

- Domain modules hold motion accumulation, calibration, bindings and preferences.
  They do not depend on WinUI controls or Bluetooth transport.
- Application services coordinate transport, reconnect policy and output routing.
  The output gate tracks actual desktop button ownership and keeps preview local.
- Infrastructure owns GATT, input injection, settings storage and Windows APIs.
- Presentation state validates drafts and handles events without native controls.
  Page modules render the state; the component coordinates UI commands and lifetime.

These boundaries follow single responsibility and dependency separation where
useful. Avoid interfaces with one implementation unless they enable a meaningful
test or isolate a platform dependency. Existing settings storage and input/profile
interfaces remain reusable. Prefer small state types over a general plugin system.

Preferences exclude connection history and calibration. Saving merges a draft
into the latest settings snapshot, validates it, persists it, then replaces the
live snapshot. A failed save retains the previous active settings and draft.
Changing bindings pauses input before the worker applies them to another packet.

## Copy rules

Use a short action label. Explain one action per sentence. Name the affected
control or system. State prerequisites and recovery steps directly. Keep the
same term for the same concept. Show raw technical details only in diagnostics.
Every catalog key supplies Chinese, English, Japanese and Korean text. These
rules are inspired by controlled technical English; they do not claim STE100
conformance or certified translations.

## Verification limits

Behavioral tests cover input timing, fractional movement, calibration failure,
reconnect limits, output routing, bindings and settings rollback. Native startup
and UI checks complement those tests. Before public release, complete physical
controller acceptance, clean Windows package testing, DPI/screen-reader review
and both UAC paths. Battery support remains unimplemented.
