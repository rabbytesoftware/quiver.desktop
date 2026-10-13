// Shared horizontal inset for the Hero and the tabs/rail content beneath it.
// Keeping this in one place is what keeps them aligned -- the same literal
// duplicated in both files is exactly how they drifted apart before.
export const CONTENT_PADDING_X = 'px-11';

// Caps how wide the page -- header included -- can stretch on very wide or
// ultrawide windows, centering it instead of letting the hero banner and the
// rail spread edge to edge. `w-full` matters here: without it, `mx-auto`'s
// auto cross-axis margins stop this flex child from stretching at all, and
// it shrinks to its content's own width instead of growing up to the cap.
export const CONTENT_MAX_WIDTH = 'w-full max-w-6xl mx-auto';
