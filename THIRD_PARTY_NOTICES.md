# Third-party notices

AudioFlow uses Tauri, windows-rs, React, Vite, Tailwind CSS, Zustand, Radix UI, Lucide, and the dependencies recorded in the committed npm and Cargo lockfiles. Their source packages contain their licenses. Tauri and windows-rs use MIT/Apache-2.0 licensing; the frontend libraries listed here use permissive licenses. Redistribution must retain applicable upstream notices.

## Adapted UI patterns

The local shadcn-style primitives follow [shadcn/ui](https://github.com/shadcn-ui/ui), copyright (c) 2023 shadcn, MIT licensed. The theme dropdown adapts [Origin UI's 21st.dev example](https://21st.dev/@originui/components/dropdown-menu/theme-toggle-with-dropdown), copyright (c) 2025 coss.com and Origin UI, MIT licensed. Both use the following grant and disclaimer:

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

## Windows policy research

The audio-policy interface IDs, ABI slots, and endpoint ID format were checked against [EarTrumpet's declarations](https://github.com/File-New-Project/EarTrumpet/tree/master/EarTrumpet/Interop/MMDeviceAPI). No EarTrumpet source files are included. Its [license](https://github.com/File-New-Project/EarTrumpet/blob/master/LICENSE) is MIT with exclusions for Yellow Elephant Productions, Tidal Media Inc., and Articent Group LLC. The Rust provider is an independent implementation of that Windows interface, with explicit restoration, compatibility failure, and observed-session verification.

[MonitorAudioRouter](https://github.com/TechlyAccurate/MonitorAudioRouter) was used as a functional reference. No source was reused because a reusable source license was not established from the inspected repository.
