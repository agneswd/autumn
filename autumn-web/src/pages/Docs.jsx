import React, { useEffect } from 'react';
import { useLocation, Link } from 'react-router-dom';

// Import MDX files globally
import SetupDocs from '../docs/setup.mdx';
import ModerationDocs from '../docs/moderation.mdx';
import CaseMgmtDocs from '../docs/case-management.mdx';
import ReversalsDocs from '../docs/reversals.mdx';
import UtilityDocs from '../docs/utility.mdx';

export default function Docs() {
    const { hash } = useLocation();

    const sections = [
        {
            id: 'setup',
            title: 'Setup & Config',
            Component: SetupDocs,
            items: [
                { id: 'modlogchannel', title: '!modlogchannel', heading: '!modlogchannel' },
                { id: 'aitoggle', title: '!aitoggle', heading: '!aitoggle' },
            ],
        },
        {
            id: 'core-mod',
            title: 'Core Moderation',
            Component: ModerationDocs,
            items: [
                { id: 'ban', title: '!ban', heading: '!ban' },
                { id: 'kick', title: '!kick', heading: '!kick' },
                { id: 'timeout', title: '!timeout', heading: '!timeout' },
                { id: 'warn', title: '!warn', heading: '!warn' },
                { id: 'purge', title: '!purge', heading: '!purge' },
                { id: 'terminate', title: '!terminate', heading: '!terminate' },
                { id: 'permissions', title: '!permissions', heading: '!permissions' },
            ],
        },
        {
            id: 'case-mgmt',
            title: 'Case Management',
            Component: CaseMgmtDocs,
            items: [
                { id: 'case', title: '!case', heading: '!case' },
                { id: 'modlogs', title: '!modlogs', heading: '!modlogs' },
                { id: 'warnings', title: '!warnings', heading: '!warnings' },
                { id: 'notes', title: '!notes', heading: '!notes' },
            ],
        },
        {
            id: 'reversals',
            title: 'Reversals',
            Component: ReversalsDocs,
            items: [
                { id: 'unban', title: '!unban', heading: '!unban' },
                { id: 'untimeout', title: '!untimeout', heading: '!untimeout' },
                { id: 'unwarn', title: '!unwarn', heading: '!unwarn' },
            ],
        },
        {
            id: 'utility',
            title: 'Utility',
            Component: UtilityDocs,
            items: [
                { id: 'help', title: '!help', heading: '!help' },
                { id: 'usage', title: '!usage', heading: '!usage' },
                { id: 'ping', title: '!ping', heading: '!ping' },
                { id: 'pagetest', title: '!pagetest', heading: '!pagetest' },
                { id: 'universe', title: '!universe', heading: '!universe' },
            ],
        },
    ];

    useEffect(() => {
        sections.forEach((section) => {
            const headings = document.querySelectorAll(`#${section.id} .prose h2`);

            headings.forEach((heading) => {
                const text = heading.textContent?.trim();
                if (!text) {
                    return;
                }

                const match = section.items.find((item) => item.heading === text);
                if (match) {
                    heading.id = match.id;
                }
            });
        });
    }, []);

    const sectionIds = sections.map((section) => section.id);
    const subcategoryIds = sections.flatMap((section) => section.items.map((item) => item.id));

    const scrollSubcategoryIntoView = (element) => {
        const targetTop = window.scrollY + element.getBoundingClientRect().top;
        const offset = window.innerHeight * 0.35;
        window.scrollTo({ top: Math.max(0, targetTop - offset), behavior: 'auto' });
    };

    const jumpToSubcategory = (event, id) => {
        event.preventDefault();
        const element = document.getElementById(id);
        if (!element) {
            return;
        }

        scrollSubcategoryIntoView(element);

        window.history.replaceState(null, '', `#${id}`);
    };

    useEffect(() => {
        if (hash) {
            const targetId = hash.replace('#', '');
            const element = document.getElementById(targetId);
            if (element) {
                setTimeout(() => {
                    if (subcategoryIds.includes(targetId)) {
                        scrollSubcategoryIntoView(element);
                        return;
                    }

                    if (sectionIds.includes(targetId)) {
                        element.scrollIntoView({ behavior: 'smooth', block: 'start' });
                    }
                }, 100);
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
                        <Link to="/docs" className="font-mono text-sm text-accent transition-colors font-bold">Commands</Link>
                        <Link to="/docs/contributing" className="font-mono text-sm text-background/60 hover:text-accent transition-colors">Contributing</Link>
                    </div>

                    <div className="h-px bg-[#1A1A1A]"></div>

                    <div className="flex flex-col gap-3">
                        <span className="font-mono text-xs text-background/40 uppercase tracking-widest">On this page</span>
                        {sections.map(sec => (
                            <div key={sec.id} className="flex flex-col gap-2">
                                <a
                                    href={`#${sec.id}`}
                                    className="font-mono text-xs text-background/60 hover:text-background transition-colors"
                                >
                                    {sec.title}
                                </a>

                                <div className="ml-3 flex flex-col gap-1 border-l border-[#1A1A1A] pl-2">
                                    {sec.items.map(item => (
                                        <a
                                            key={item.id}
                                            href={`#${item.id}`}
                                            onClick={(event) => jumpToSubcategory(event, item.id)}
                                            className="font-mono text-[11px] text-background/45 hover:text-accent transition-colors"
                                        >
                                            {item.title}
                                        </a>
                                    ))}
                                </div>
                            </div>
                        ))}
                    </div>
                </div>
            </aside>

            {/* Main Content */}
            <div className="flex-1 pt-32 pb-24 px-6 lg:px-16 max-w-4xl mx-auto">
                <h1 className="font-sans text-4xl font-bold text-background mb-4">Command Reference</h1>
                <p className="font-mono text-background/60 leading-relaxed mb-12">
                    Autumn is designed as a prefix-first bot (`!`), but natively supports Discord Slash Commands for all inputs. The documentation below covers standard usage patterns.
                </p>

                <div className="flex flex-col gap-16">
                    {sections.map(sec => (
                        <section key={sec.id} id={sec.id} className="scroll-mt-32">
                            <div className="prose prose-invert prose-pre:bg-[#0A0A0A] prose-pre:border prose-pre:border-[#333] prose-headings:font-sans prose-headings:text-background prose-a:text-accent max-w-none">
                                <sec.Component />
                            </div>
                        </section>
                    ))}
                </div>
            </div>

            {/* Balancing Spacer for Global Centering */}
            <div className="hidden lg:block w-64 shrink-0"></div>
        </div>
    );
}
