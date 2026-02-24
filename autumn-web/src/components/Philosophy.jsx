import React, { useEffect, useRef } from 'react';
import gsap from 'gsap';
import { ScrollTrigger } from 'gsap/ScrollTrigger';

export default function Philosophy() {
    const containerRef = useRef(null);
    const contentRef = useRef(null);

    useEffect(() => {
        const ctx = gsap.context(() => {
            gsap.fromTo(contentRef.current,
                { autoAlpha: 0, y: 30 },
                {
                    autoAlpha: 1,
                    y: 0,
                    duration: 1,
                    ease: "power3.out",
                    scrollTrigger: {
                        trigger: contentRef.current,
                        start: "top 80%",
                    }
                }
            );
        }, containerRef);
        return () => ctx.revert();
    }, []);

    return (
        <section ref={containerRef} className="relative w-full pt-24 pb-10 bg-primary flex flex-col items-center border-t border-[#1A1A1A]">
            <div ref={contentRef} className="w-full max-w-4xl mx-auto px-6">

                <div className="mb-12 flex flex-col items-center text-center">
                    <h2 className="font-mono font-bold text-sm text-accent tracking-widest uppercase mb-2">&gt;_ WHY AUTUMN?</h2>
                    <h3 className="font-sans text-2xl md:text-3xl font-medium text-background">Pure Utility. Zero Bloat.</h3>
                </div>

                <div className="bg-[#0A0A0A] rounded-lg border border-[#333] overflow-hidden">
                    <div className="bg-[#151515] border-b border-[#333] px-4 py-2 flex items-center justify-between">
                        <div className="flex items-center gap-2">
                            <div className="w-2.5 h-2.5 rounded-full bg-[#ff5f56]"></div>
                            <div className="w-2.5 h-2.5 rounded-full bg-[#ffbd2e]"></div>
                            <div className="w-2.5 h-2.5 rounded-full bg-[#27c93f]"></div>
                        </div>
                        <span className="font-mono text-xs text-background/40">philosophy.diff</span>
                    </div>

                    <div className="p-6 overflow-x-auto">
                        <pre className="font-mono text-xs md:text-sm leading-relaxed whitespace-pre-wrap break-words">
                            <span className="text-[#ff5f56] block">- Most bots try to do everything (economy, leveling, music).</span>
                            <span className="text-[#ff5f56] block">- They are closed source and monetize community management.</span>
                            <span className="text-[#ff5f56] block">- Hosted on mystery servers with unknown uptime.</span>
                            <span className="text-[#555] block my-2">@@ -1,3 +1,3 @@</span>
                            <span className="text-[#27c93f] block">+ Autumn focuses strictly on moderation utilities.</span>
                            <span className="text-[#27c93f] block">+ Public code. You see exactly what it does and how it runs.</span>
                            <span className="text-[#27c93f] block">+ Self-hosted. Your community, your data, your infrastructure.</span>
                        </pre>
                    </div>
                </div>

            </div>
        </section>
    );
}
