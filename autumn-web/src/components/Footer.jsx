import React from 'react';
import { Link } from 'react-router-dom';
import { Terminal } from 'lucide-react';

export default function Footer() {
    return (
        <footer className="w-full bg-[#050505] pt-16 pb-8 px-6 relative z-20 overflow-hidden border-t border-[#1A1A1A]">
            <div className="max-w-5xl mx-auto flex flex-col items-center">

                <div className="flex items-center justify-center gap-3 mb-10 group cursor-default">
                    <Terminal className="w-6 h-6 text-accent" />
                    <h2 className="font-mono font-bold text-xl text-background tracking-wide">Autumn</h2>
                </div>

                <div className="flex flex-col md:flex-row items-center justify-between w-full border-t border-[#1A1A1A] pt-8">
                    <p className="font-mono text-xs text-background/50 mb-4 md:mb-0">
                        © {new Date().getFullYear()} agneswd - MIT License
                    </p>

                    <div className="flex gap-6 font-mono text-xs">
                        <Link to="/docs" className="text-background/50 hover:text-accent hover:-translate-y-px transition-all">Docs</Link>
                        <Link to="/docs/contributing" className="text-background/50 hover:text-accent hover:-translate-y-px transition-all">Contributing</Link>
                        <a href="https://github.com/agneswd/autumn" target="_blank" rel="noreferrer" className="text-background/50 hover:text-accent hover:-translate-y-px transition-all">GitHub</a>
                    </div>
                </div>
            </div>
        </footer>
    );
}
