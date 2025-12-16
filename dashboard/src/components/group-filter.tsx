import { useEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent as ReactKeyboardEvent } from "react";
import PeoplesIcon from "@rsuite/icons/Peoples";

import * as s from "./group-filter.module.css";
import {Button} from "rsuite";

type GroupFilterProps = {
    groups: string[];
    value: string | null;
    onChange: (group: string | null) => void;
};

export const GroupFilter = ({ groups, value, onChange }: GroupFilterProps) => {
    const [open, setOpen] = useState(false);
    const [query, setQuery] = useState("");
    const wrapperRef = useRef<HTMLDivElement | null>(null);
    const inputRef = useRef<HTMLInputElement | null>(null);

    useEffect(() => {
        if (!open) {
            return;
        }
        const onClickOutside = (evt: MouseEvent) => {
            if (wrapperRef.current && !wrapperRef.current.contains(evt.target as Node)) {
                setOpen(false);
            }
        };
        const onKey = (evt: KeyboardEvent) => {
            if (evt.key === "Escape") {
                setOpen(false);
            }
        };
        document.addEventListener("mousedown", onClickOutside);
        document.addEventListener("keydown", onKey);
        return () => {
            document.removeEventListener("mousedown", onClickOutside);
            document.removeEventListener("keydown", onKey);
        };
    }, [open]);

    useEffect(() => {
        if (open) {
            // if input is not in the dom yet, this crashes without setTimeout
            setTimeout(() => inputRef.current?.focus(), 0);
        }
    }, [open]);

    const normalizedGroups = useMemo(() => {
        const uniq = new Set<string>();
        groups.forEach((g) => {
            if (g) {
                uniq.add(g);
            }
        });
        return Array.from(uniq).sort((a, b) => a.localeCompare(b));
    }, [groups]);

    const filteredGroups = useMemo(() => {
        const trimmed = query.trim().toLowerCase();
        if (!trimmed) {
            return normalizedGroups;
        }
        return normalizedGroups.filter((g) => g.toLowerCase().includes(trimmed));
    }, [normalizedGroups, query]);

    const handleSelect = (group: string | null) => {
        onChange(group);
        setOpen(false);
        setQuery("");
    };

    const handleEnter = (evt: ReactKeyboardEvent<HTMLInputElement>) => {
        if (evt.key === "Enter") {
            evt.preventDefault();
            const trimmed = query.trim();
            handleSelect(trimmed || null);
        }
    };

    return (
        <div className={s.Wrapper} ref={wrapperRef}>
            <Button
                aria-haspopup="dialog"
                aria-expanded={open}
                aria-label="Filter tasks by group"
                className={`${s.Trigger} ${open ? s.TriggerOpen : ""} ${value ? s.TriggerActive : ""}`}
                onClick={() => setOpen((o) => !o)}
            >
                <PeoplesIcon />
                <span className={s.TriggerText}>{value ?? "Groups"}</span>
            </Button>
            {open ? (
                <div className={s.Popover} role="dialog" aria-label="Select a task group">
                    <div className={s.SearchRow}>
                        <input
                            ref={inputRef}
                            className={s.SearchInput}
                            placeholder="Type to filter group"
                            value={query}
                            onChange={(evt) => setQuery(evt.target.value)}
                            onKeyDown={handleEnter}
                        />
                        {value ? (
                            <button
                                type="button"
                                className={s.ClearButton}
                                onClick={() => handleSelect(null)}
                                aria-label="Clear group filter"
                            >
                                Clear
                            </button>
                        ) : null}
                    </div>
                    <div className={s.GroupList}>
                        <button
                            type="button"
                            className={`${s.Option} ${value === null ? s.OptionActive : ""}`}
                            onClick={() => handleSelect(null)}
                        >
                            <span className={s.OptionLabel}>All groups</span>
                        </button>
                        {filteredGroups.map((groupName) => (
                            <button
                                type="button"
                                key={groupName}
                                className={`${s.Option} ${value === groupName ? s.OptionActive : ""}`}
                                onClick={() => handleSelect(groupName)}
                            >
                                <span className={s.OptionLabel}>{groupName}</span>
                            </button>
                        ))}
                        {filteredGroups.length === 0 ? (
                            <div className={s.EmptyState}>
                                <span>No matching groups</span>
                                {query.trim() ? (
                                    <span className={s.HintText}>Press Enter to use "{query.trim()}" anyways</span>
                                ) : null}
                            </div>
                        ) : null}
                    </div>
                </div>
            ) : null}
        </div>
    );
};
