import React, { useEffect } from 'react';
import { useLocation, Link } from 'react-router-dom';
import ContributingDocs from '../docs/contributing.mdx';

export default function Contributing() {
    const { hash } = useLocation();

    const sections = [
        { id: 'how-to-contribute', title: 'How to Contribute' },
        { id: 'community-guidelines', title: 'Community Guidelines' },
        { id: 'support-the-project', title: 'Support the Project' },
        { id: 'license', title: 'License' },
        { id: 'getting-help', title: 'Getting Help' }
    ];

    useEffect(() => {
        const h2Nodes = document.querySelectorAll('.prose h2');
        h2Nodes.forEach((heading) => {
            const text = heading.textContent?.trim();
            if (!text) {
                return;
            }

            const matching = sections.find((section) => section.title === text);
            if (matching) {
                heading.id = matching.id;
            }
        });
    }, []);

    useEffect(() => {
        if (hash) {
            const element = document.getElementById(hash.replace('#', ''));
            if (element) {
                setTimeout(() => element.scrollIntoView({ behavior: 'smooth' }), 100);
            }
        } else {
            window.scrollTo(0, 0);
        }
    }, [hash]);

    return (
        <div className="w-full flex">
            {/* Sidebar Navigation */}
            <aside className="hidden lg:block w-64 shrink-0 pt-32 pb-24 border-r border-[#1A1A1A] sticky top-0 h-screen overflow-y-auto">
                <div className="px-6 flex flex-col gap-6">
                    <div className="flex flex-col gap-2">
                        <Link to="/" className="font-mono text-sm text-background/60 hover:text-accent transition-colors">Home</Link>
                        <Link to="/docs" className="font-mono text-sm text-background/60 hover:text-accent transition-colors">Commands</Link>
                        <Link to="/docs/contributing" className="font-mono text-sm text-accent transition-colors font-bold">Contributing</Link>
                    </div>

                    <div className="h-px bg-[#1A1A1A]"></div>

                    <div className="flex flex-col gap-3">
                        <span className="font-mono text-xs text-background/40 uppercase tracking-widest">On this page</span>
                        {sections.map(sec => (
                            <a
                                key={sec.id}
                                href={`#${sec.id}`}
                                className="font-mono text-xs text-background/60 hover:text-background transition-colors"
                            >
                                {sec.title}
                            </a>
                        ))}
                    </div>
                </div>
            </aside>

            {/* Main Content */}
            <div className="flex-1 pt-32 pb-24 px-6 lg:px-16 max-w-4xl mx-auto">
                <div className="prose prose-invert prose-headings:scroll-mt-32 prose-pre:bg-[#0A0A0A] prose-pre:border prose-pre:border-[#333] prose-headings:font-sans prose-headings:text-background prose-a:text-accent prose-p:font-mono prose-p:text-background/80 prose-li:font-mono prose-li:text-background/80 max-w-none">
                    <ContributingDocs />
                </div>
            </div>

            {/* Balancing Spacer for Global Centering */}
            <div className="hidden lg:block w-64 shrink-0"></div>
        </div>
    );
}
