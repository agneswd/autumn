import React, { useEffect, useRef } from 'react';
import gsap from 'gsap';
import { Terminal, Copy, Check } from 'lucide-react';
import { useState } from 'react';

export default function Hero() {
    const [copied, setCopied] = useState(false);
    const containerRef = useRef(null);

    const handleCopy = () => {
        navigator.clipboard.writeText("git clone https://github.com/agneswd/autumn.git");
        setCopied(true);
        setTimeout(() => setCopied(false), 2000);
    };

    useEffect(() => {
        const ctx = gsap.context(() => {
            gsap.set('.hero-reveal', { y: 20, opacity: 0 });
            gsap.to('.hero-reveal', {
                y: 0,
                opacity: 1,
                duration: 0.8,
                stagger: 0.1,
                ease: "power2.out",
                delay: 0.1
            });
        }, containerRef);
        return () => ctx.revert();
    }, []);

    return (
        <section ref={containerRef} className="w-full flex flex-col items-center justify-center overflow-x-hidden pt-32 pb-24 px-4 sm:px-6">

            <div className="hero-reveal w-full max-w-4xl overflow-x-hidden select-none mb-12 flex justify-center text-accent text-glow-accent">
                <pre className="max-w-full overflow-hidden font-mono text-[clamp(3px,1vw,14px)] leading-tight tracking-[-0.01em]">
                    {` █████╗ ██╗   ██╗████████╗██╗   ██╗███╗   ███╗███╗   ██╗
██╔══██╗██║   ██║╚══██╔══╝██║   ██║████╗ ████║████╗  ██║
███████║██║   ██║   ██║   ██║   ██║██╔████╔██║██╔██╗ ██║
██╔══██║██║   ██║   ██║   ██║   ██║██║╚██╔╝██║██║╚██╗██║
██║  ██║╚██████╔╝   ██║   ╚██████╔╝██║ ╚═╝ ██║██║ ╚████║
╚═╝  ╚═╝ ╚═════╝    ╚═╝    ╚═════╝ ╚═╝     ╚═╝╚═╝  ╚═══╝`}
                </pre>
            </div>

            <div className="hero-reveal text-center max-w-2xl mx-auto mb-16">
                <h1 className="font-sans text-xl md:text-2xl text-background font-medium mb-4">
                    A general-purpose Discord moderation bot.
                </h1>
                <p className="font-mono text-background/60 text-sm md:text-base leading-relaxed">
                    Written in Rust, Serenity, and Poise.<br />
                    Made for fun, private use, and open source exploration.
                </p>
            </div>

            <div className="hero-reveal w-full max-w-2xl mx-auto">
                <div className="flex items-center gap-2 mb-3 px-1">
                    <Terminal className="w-4 h-4 text-accent" />
                    <span className="font-mono text-xs text-background/50 uppercase tracking-wider">Quickstart</span>
                </div>

                <div className="bg-[#0A0A0A] border border-white/10 rounded-lg p-1 relative group hover:border-accent/50 transition-colors">
                    <div className="flex items-center justify-between bg-[#121212] rounded py-4 px-4 md:px-6 gap-4">
                        <code className="font-mono text-xs sm:text-sm md:text-base text-background/90 group-hover:text-white transition-colors overflow-x-auto whitespace-nowrap scrollbar-hide">
                            <span className="text-accent inline-block mr-2 md:mr-3 select-none">$</span>
                            git clone https://github.com/agneswd/autumn.git
                        </code>
                        <button
                            onClick={handleCopy}
                            className="shrink-0 bg-[#1A1A1A] hover:bg-[#252525] p-2 rounded transition-colors text-background/50 hover:text-accent"
                            title="Copy to clipboard"
                        >
                            {copied ? <Check className="w-4 h-4 text-green-500" /> : <Copy className="w-4 h-4" />}
                        </button>
                    </div>
                </div>
            </div>

        </section>
    );
}
