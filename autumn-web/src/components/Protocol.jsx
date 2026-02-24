import React, { useEffect, useRef } from 'react';
import gsap from 'gsap';
import { ScrollTrigger } from 'gsap/ScrollTrigger';

export default function Protocol() {
    const containerRef = useRef(null);
    const cardsRef = useRef([]);

    const steps = [
        {
            num: "01",
            title: "Clone & Compile",
            desc: "Pull the repo and compile from source using cargo. Zero black-box binaries.",
            Animation: () => (
                <div className="w-full font-mono text-xs md:text-sm text-[#27c93f] flex flex-col gap-2">
                    <div>❯ cargo build</div>
                    <div className="text-[#ffbd2e]">   Compiling autumn-utils v0.1.0</div>
                    <div className="text-[#ffbd2e]">   Compiling autumn-commands v0.1.0</div>
                    <div className="text-[#ffbd2e]">   Compiling autumn-bot v0.1.0</div>
                    <div className="text-[#27c93f]">    Finished `dev` profile [unoptimized + debuginfo] target(s)</div>
                </div>
            )
        },
        {
            num: "02",
            title: "Configure Variables",
            desc: "Setup your environment variables. Define the keys and basic parameters.",
            Animation: () => (
                <div className="w-full font-mono text-xs md:text-sm text-background/80 flex flex-col gap-2">
                    <div className="text-[#ff5f56]"># .env</div>
                    <div>DISCORD_TOKEN="MTR..."</div>
                    <div>OWNER_ID="123456789"</div>
                    <div>DATABASE_URL="postgresql://user:pass@localhost/db"</div>
                </div>
            )
        },
        {
            num: "03",
            title: "Run Binary",
            desc: "Execute the compiled binary. The bot connects, syncs slash commands, and listens.",
            Animation: () => (
                <div className="w-full font-mono text-xs md:text-sm text-background/80 flex flex-col gap-2">
                    <div className="text-background">Running `target/debug/autumn-bot`</div>
                    <div className="text-background/60">2026-02-22T15:38:05.357Z <span className="text-[#27c93f]">INFO</span> autumn_bot: PostgreSQL connection established.</div>
                    <div className="text-background/60">2026-02-22T15:38:05.387Z <span className="text-[#27c93f]">INFO</span> autumn_bot: Autumn is connecting...</div>
                    <div className="text-background/60">2026-02-22T15:38:06.543Z <span className="text-[#27c93f]">INFO</span> autumn_bot: Autumn has awoken!</div>
                    <div className="text-[#ffbd2e] animate-pulse">_</div>
                </div>
            )
        }
    ];

    useEffect(() => {
        const ctx = gsap.context(() => {
            // New sticky-based stacking logic (no GSAP pin)
            cardsRef.current.forEach((card, index) => {
                if (index === cardsRef.current.length - 1) return;

                const innerCard = card.querySelector('.protocol-card-inner');
                const nextCard = cardsRef.current[index + 1];

                if (innerCard && nextCard) {
                    gsap.to(innerCard, {
                        scale: 0.9,
                        opacity: 0.3,
                        filter: "blur(4px)",
                        ease: "none",
                        scrollTrigger: {
                            trigger: nextCard,
                            start: "top bottom",
                            end: "top top",
                            scrub: true,
                        }
                    });
                }
            });
        }, containerRef);
        return () => ctx.revert();
    }, []);

    return (
        <section id="protocol" className="w-full bg-primary relative" ref={containerRef}>
            {steps.map((step, index) => (
                <div
                    key={index}
                    ref={el => cardsRef.current[index] = el}
                    className="h-screen w-full flex items-center justify-center sticky top-0 overflow-hidden bg-primary"
                    style={{ zIndex: index }}
                >
                    <div className="protocol-card-inner h-[390px] md:h-[330px] w-[90vw] max-w-4xl bg-[#0A0A0A] rounded-lg border border-[#333] flex flex-col md:flex-row shadow-[0_0_30px_rgba(0,0,0,0.8)] overflow-hidden">

                        {/* Terminal Window Decoration */}
                        <div className="absolute top-0 left-0 w-full h-8 bg-[#151515] border-b border-[#333] px-4 flex items-center gap-2 md:hidden">
                            <div className="w-2.5 h-2.5 rounded-full bg-[#ff5f56]"></div>
                            <div className="w-2.5 h-2.5 rounded-full bg-[#ffbd2e]"></div>
                            <div className="w-2.5 h-2.5 rounded-full bg-[#27c93f]"></div>
                            <span className="font-mono text-[10px] text-background/40 ml-2">step_0{index + 1}.sh</span>
                        </div>

                        {/* Content Panel */}
                        <div className="flex-1 md:basis-1/2 flex flex-col justify-center p-8 md:p-16 pt-16 md:pt-16 border-b md:border-b-0 md:border-r border-[#333] bg-[#0E0E0E] overflow-y-auto">
                            <span className="font-mono text-sm text-accent mb-4 font-bold">[{step.num}]</span>
                            <h3 className="font-sans font-medium text-2xl md:text-3xl text-background mb-4">
                                {step.title}
                            </h3>
                            <p className="font-mono text-sm text-background/60 leading-relaxed">
                                {step.desc}
                            </p>
                        </div>

                        {/* Visual / Code Panel */}
                        <div className="flex-1 md:basis-1/2 flex flex-col bg-[#050505] relative">
                            {/* Desktop Terminal Header */}
                            <div className="hidden md:flex w-full h-8 bg-[#151515] border-b border-[#333] px-4 items-center gap-2">
                                <div className="w-2.5 h-2.5 rounded-full bg-[#ff5f56]"></div>
                                <div className="w-2.5 h-2.5 rounded-full bg-[#ffbd2e]"></div>
                                <div className="w-2.5 h-2.5 rounded-full bg-[#27c93f]"></div>
                                <span className="font-mono text-[10px] text-background/40 ml-2">step_0{index + 1}.sh</span>
                            </div>
                            <div className="p-8 flex-1 flex items-center overflow-auto">
                                <step.Animation />
                            </div>
                        </div>

                    </div>
                </div>
            ))}
        </section>
    );
}
