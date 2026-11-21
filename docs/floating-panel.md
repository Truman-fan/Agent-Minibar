# Floating usage panel

Enable **Settings > Customize > Floating panel** to detach the popup from the
taskbar. Enabling it also pins the panel open. Drag the dedicated header to move
it, including across monitors. Tabs and action buttons remain interactive.
Both modes show a compact bar with the app logo, Agent Minibar title and action
buttons on the same side as the tabs. It sits below the tabs for bottom-anchored
popups and above them for top-anchored popups. Only the floating mode permits
dragging its title area; the regular popup remains anchored to the taskbar.
Edit Home opens the Home editor and becomes Done while editing. The Show edit
button preference applies to this bar; Done remains available while editing.
**Settings > Customize > Layout > Show popup action bar** hides or shows this
bar in the regular popup only. Floating panels always keep it. The change is
saved and animates immediately. With the bar hidden, pin and Edit/Done return
to the tab bar; Edit follows its own visibility preference, and Done always
remains available during editing. With the action bar visible, these controls
are not duplicated in the tab bar.
The pin button next to the close button controls outside-click and Escape
dismissal. The close button hides the panel without clearing its pin or position;
the tray icon and popup hotkey also hide or reopen it. Header buttons do not drag
the window.

The two-column button next to the pin changes **Settings > Customize > Use two
columns** immediately and saves the preference. Its accent state reflects that
setting, including changes made in Settings. Home and Usage follow the toggle;
provider tabs keep their single-column layout.

**Always on top** and the popup pin control the same persisted preference.
Enabling either keeps the panel above other applications and prevents dismissal
on focus loss, outside clicks, and Escape. Disabling either restores dismissal
when another application takes the foreground or the user clicks outside.
Settings remains a live preview surface and does not trigger dismissal.
Notifications keep their own topmost behavior. An unpinned popup still opens in
front, but only a pinned panel stays in the topmost band. Changes apply to an
open panel without restarting.

The panel grows downward from its saved top edge. Its maximum height is based
on the monitor size, independently of its position; overflowing content scrolls.
With two columns enabled, Home and Usage use two columns. Provider tabs resize
the visible window to one column. Resizing preserves the top-left origin and
can extend beyond the screen edge; the app never snaps the panel into a work area.
Header and content size changes follow
the application's and Windows' animation preferences.

The saved position restores the exact screen coordinates, including positions
partially outside a monitor. An open panel uses its live native window origin
when resizing or processing display changes. Monitor and work-area metadata in
older settings remains compatible, but does not override the screen coordinates.
**Reset position** clears the saved origin and places the panel at the upper-right
of the current monitor. Disable **Floating panel** to restore tray positioning;
the saved floating position remains available when it is enabled again.

The tray/worker bridge uses its idle polling cadence while a pinned panel is
open and Settings is closed. GPUI drives animations itself and worker updates
continue to publish normally.

## Floating circle (Windows)

New Windows installations start with **Settings > Customize > Floating circle**
enabled. Existing installations retain their saved on/off choice. The compact
96 by 122 DIP quota panel does not depend on Explorer's taskbar and remains topmost when
the taskbar is auto-hidden or unavailable. The first visible configured taskbar
widget supplies its provider and indicators; when no taskbar widget is
configured, the first enabled provider supplies the defaults. Two thick status
rings sit above a flat two-column information card. Ring and percentage colors
follow the remaining quota, while the first and second labels keep blue and
green identity fills. A neutral circle separates two available quota rings
without representing progress. Providers without percentage quotas show one
thin neutral placeholder circle instead; its outer edge matches the ordinary
outer quota ring and its slightly larger provider mark keeps the empty state
balanced. The placeholder never appears alongside a quota ring. Its lower card
keeps the same filled-title format as percentage quotas and shows a real
balance, unlimited status, authorization problem, temporary failure, loading
state, or the absence of a quota API. Consumption totals never become quota
rings, and an unavailable read is never presented as 0%. Cached quota values
remain visible but dim and receive a warning marker after a failed refresh.
The panel keeps at most two indicators and follows the taskbar's light/dark
appearance. Customize exposes 40–100% surface opacity. The information card
follows the circle opacity by default and can be adjusted independently; quota
progress, provider marks, labels and values do not fade with the surface. The
provider glyph is opaque, with a fine two-tone outline around its silhouette
and a transparent center behind it. Light
background enhancement is enabled by default and adds a paired keyline around
the neutral rings and cards.

A red light below the quota card summarizes confirmed Codex model reroutes
for the circle's profile. It counts conversations. Open the detail card to
see the requested and redirected model, the official reason, and whether the
round has ended. A confirmed notice remains visible until marked as viewed.

Live monitoring requires an already running shared Codex App Server with the
default control socket for that profile. For example, a compatible CLI can
connect using `codex --remote unix://` to an App Server running with
`codex app-server --listen unix://` under the same `CODEX_HOME`.
Minibar does not start a server or send model prompts. Standalone sessions
without shared event transport show the unavailable status in details.

One floating circle is shown by default. An optional second circle can be
enabled from Customize or from the first circle's detail card. The circles are
always a fixed vertical pair with a 10 DIP gap, move as one group, and are
clamped as a group so they cannot overlap or split across an edge. More than
two circles are never rendered. Each slot independently binds to one enabled
provider account. Agents, child Agents and CLI sessions can contribute to
separate usage statistics, but are never floating-circle targets and never
create circles automatically.

On Windows, an automatically detected Codex account pair gives the desktop
account priority when the first circle follows the default Codex target.
Adding a second circle then offers the separate Codex CLI login by default.
An explicitly selected target takes precedence. Detection at startup requires
both installations, the standard desktop profile, and an independent CLI
`CODEX_HOME` inherited by Minibar with a different ChatGPT login. Native clients
sharing one login have one target. When an override is private to a terminal,
add a Codex instance in **Settings > Providers** and point **Config folder**
at that terminal's `CODEX_HOME`, then select the instance under **Customize**.

Drag the circle to move it across monitors. Releasing a drag saves its physical
screen origin immediately. **Reset position** places it at the upper-right of
the display containing the pointer. Left-click a circle to open its provider's
detail card, click the other circle to switch cards, or click the current circle
again to close the detail card. The detail card and Settings cover the floating
circles where they overlap. A circle returns to the topmost band when Settings
moves away, minimizes or closes. The popup footer's red **Exit** button shuts the application
down through the normal worker and persistence cleanup path. Right-clicking
opens Settings. The lower quota card uses a dark translucent surface, compact
blue and green label capsules, a centered divider, and a paired keyline so its
semi-transparent values remain legible over both white documents and dark
editors. Quota numbers use a fine dark outline, which switches to light gray
for very dark quota colors. Wide Settings windows reserve an animated strip on
the screen-right side while the circle is enabled, keeping page controls clear
of the default floating position after maximize. The native window unions a
circular ring region with the lower rounded card and, only while present, the
alert card. Transparent space outside those shapes neither blocks nor receives
clicks. With two circles, each native window uses its own current height, so an
alert on one account does not create a blank gap inside the other.

When the popup was opened by a floating circle, its footer adds **Switch
target**. This opens Customize directly at the visible target selectors. The
first circle also offers **Add second** while no second slot exists; the second
circle offers **Close circle**. These actions dismiss the detail card before
changing the circle layout. Ordinary tray/taskbar popups do not show these
slot-specific actions.

## Height resizing

Both the tray popup and floating panel support manual height resizing. Drag the
free vertical edge: the top edge for a bottom-anchored tray popup, or the bottom
edge for a top-anchored popup. Floating panels can be resized from either edge.
The opposite edge stays fixed, including when resizing a floating panel upward.
The height follows the pointer directly and is saved when the drag finishes;
the same height preference applies across views, modes, and application restarts.
The maximum is the smaller of the owning monitor's full screen height and the
current tab's measured content plus window chrome. Switching to shorter content
automatically removes unused space while retaining the preferred height for
longer tabs. Content exceeding the selected height remains scrollable.
Width continues to follow each tab's column layout. Normal content/layout
transitions still respect the animation preferences.

## Verification

Unit tests cover old settings defaults, persistence, negative and off-screen
origins, fixed origins across column counts and DPI values, and height
independence from work-area edges. Native drag, stacking, and physical monitor
transitions require manual verification; automated checks do not launch the app.
