import React, { useEffect, useRef, useState } from 'react';
import { Link } from 'react-router-dom';
import gsap from 'gsap';
import { Github, Terminal, Menu, X } from 'lucide-react';

export default function Navbar() {
    const navRef = useRef(null);
    const [isOpen, setIsOpen] = useState(false);

    useEffect(() => {
        const ctx = gsap.context(() => {
            // Morphing Logic: Transparent -> Blurred background on scroll
            gsap.to(navRef.current, {
                scrollTrigger: {
                    trigger: "body",
                    start: "top -100px",
                    end: "+=1",
                    toggleActions: "play none none reverse",
                    onEnter: () => navRef.current.classList.add('bg-primary/80', 'backdrop-blur-xl', 'border-dark'),
                    onLeaveBack: () => navRef.current.classList.remove('bg-primary/80', 'backdrop-blur-xl', 'border-dark'),
                }
            });
        }, navRef);
        return () => ctx.revert();
    }, []);

    return (
        <nav
            ref={navRef}
            className="fixed top-6 left-1/2 -translate-x-1/2 z-40 w-full max-w-5xl rounded-lg border border-transparent transition-colors duration-300"
        >
            <div className="flex items-center justify-between px-6 py-4">
                <Link to="/" className="flex items-center gap-2 group" onClick={() => setIsOpen(false)}>
                    <Terminal className="w-5 h-5 text-accent transition-transform group-hover:scale-110" />
                    <span className="font-sans font-bold text-lg tracking-tight text-background">Autumn</span>
                </Link>

                {/* Desktop Menu */}
                <div className="hidden md:flex items-center gap-8 font-mono text-sm text-background/70">
                    <Link to="/#features" className="hover:text-accent hover:-translate-y-[1px] transition-all">Features</Link>
                    <Link to="/#protocol" className="hover:text-accent hover:-translate-y-[1px] transition-all">Protocol</Link>
                    <Link to="/docs" className="hover:text-accent hover:-translate-y-[1px] transition-all">Docs</Link>
                </div>

                <div className="hidden md:flex">
                    <a
                        href="https://github.com/agneswd/autumn"
                        target="_blank"
                        rel="noreferrer"
                        className="group relative overflow-hidden bg-white/10 hover:bg-white/20 text-background px-5 py-2 rounded-full font-sans text-sm font-medium transition-all duration-300 hover:scale-[1.03] flex items-center gap-2"
                        style={{ transitionTimingFunction: 'cubic-bezier(0.25, 0.46, 0.45, 0.94)' }}
                    >
                        <span className="relative z-10 flex items-center gap-2">
                            <Github className="w-4 h-4" />
                            Source
                        </span>
                        <span className="absolute inset-0 bg-accent translate-y-[100%] group-hover:translate-y-0 transition-transform duration-300 ease-in-out z-0"></span>
                    </a>
                </div>

                {/* Mobile Hamburger Button */}
                <button 
                    className="md:hidden text-background p-2"
                    onClick={() => setIsOpen(!isOpen)}
                >
                    {isOpen ? <X className="w-6 h-6" /> : <Menu className="w-6 h-6" />}
                </button>
            </div>

            {/* Mobile Menu Dropdown */}
            {isOpen && (
                <div className="md:hidden absolute top-full left-0 w-full mt-2 bg-primary/95 backdrop-blur-xl border border-dark rounded-lg p-4 flex flex-col gap-4 shadow-xl">
                    <Link to="/#features" onClick={() => setIsOpen(false)} className="font-mono text-sm text-background/80 hover:text-accent p-2">Features</Link>
                    <Link to="/#protocol" onClick={() => setIsOpen(false)} className="font-mono text-sm text-background/80 hover:text-accent p-2">Protocol</Link>
                    <Link to="/docs" onClick={() => setIsOpen(false)} className="font-mono text-sm text-background/80 hover:text-accent p-2">Docs</Link>
                    <a
                        href="https://github.com/agneswd/autumn"
                        target="_blank"
                        rel="noreferrer"
                        className="flex items-center justify-center gap-2 bg-white/10 text-background px-5 py-3 rounded-lg font-sans text-sm font-medium mt-2"
                    >
                        <Github className="w-4 h-4" />
                        Source
                    </a>
                </div>
            )}
        </nav>
    );
}
