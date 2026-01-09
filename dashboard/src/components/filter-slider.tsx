import { CSSProperties, useEffect, useMemo, useRef, useState } from "react";

import * as s from "./filter-slider.module.css";
import {FilterOption, FilterValue} from "./tasks";

type DragState = { startX: number; moved: boolean };

type FilterSliderProps = {
    options: FilterOption[];
    value: FilterValue;
    onChange: (value: FilterValue) => void;
};

const hexToRgb = (hex: string) => {
    const clean = hex.replace("#", "");
    const num = parseInt(clean, 16);
    return {
        r: (num >> 16) & 255,
        g: (num >> 8) & 255,
        b: num & 255,
    };
};

const mixColors = (a: string, b: string, t: number) => {
    const ca = hexToRgb(a);
    const cb = hexToRgb(b);
    const lerp = (x: number, y: number) => Math.round(x + (y - x) * t);
    const r = lerp(ca.r, cb.r);
    const g = lerp(ca.g, cb.g);
    const bVal = lerp(ca.b, cb.b);
    const toHex = (n: number) => n.toString(16).padStart(2, "0");
    return `#${toHex(r)}${toHex(g)}${toHex(bVal)}`;
};

const withAlpha = (hex: string, alpha: number) => {
    const { r, g, b } = hexToRgb(hex);
    return `rgba(${r}, ${g}, ${b}, ${alpha})`;
};

export const FilterSlider = ({ options, value, onChange }: FilterSliderProps) => {
    const colors = options.map((option) => option.color);

    const [isDropdownOpen, setDropdownOpen] = useState(false);
    const [dragOffset, setDragOffset] = useState(0);
    const [dragState, setDragState] = useState<DragState | null>(null);
    const [hovering, setHovering] = useState(false);
    const sliderRef = useRef<HTMLDivElement | null>(null);
    const wrapperRef = useRef<HTMLDivElement | null>(null);

    const selectedIndex = useMemo(() => {
        const idx = options.findIndex(f => f.value === value);
        return idx >= 0 ? idx : 0;
    }, [options, value]);

    useEffect(() => {
        if (!isDropdownOpen) {
            return;
        }
        const handler = (e: MouseEvent) => {
            if (wrapperRef.current && !wrapperRef.current.contains(e.target as Node)) {
                setDropdownOpen(false);
            }
        };
        document.addEventListener("click", handler);
        return () => document.removeEventListener("click", handler);
    }, [isDropdownOpen]);

    const clampIndex = (target: number) => {
        return Math.max(0, Math.min(options.length - 1, target));
    };

    const applySelection = (targetIndex: number) => {
        if (!options.length) {
            return;
        }
        const clamped = clampIndex(targetIndex);
        onChange(options[clamped].value);
        setDragState(null);
        setDragOffset(0);
    };

    const borderOverlayStyle = useMemo<CSSProperties>(() => {
        const nextIdx = Math.max(0, Math.min(colors.length - 1, Math.ceil(selectedIndex - dragOffset)));
        const prevIdx = Math.max(0, Math.min(colors.length - 1, Math.floor(selectedIndex - dragOffset)));
        const fallbackColor = colors[0] ?? "#6b7280";
        const currentColor = colors[prevIdx] ?? fallbackColor;
        const nextColor = colors[nextIdx] ?? fallbackColor;
        const midColor = mixColors(currentColor, nextColor, 0.5);
        const gradient = `linear-gradient(120deg, ${withAlpha(currentColor, 0.55)} 0%, ${withAlpha(midColor, 0.65)} 50%, ${withAlpha(nextColor, 0.55)} 100%)`;

        const active = hovering || !!dragState;
        if (!active) {
            return {
                borderColor: "var(--rs-border-primary)",
                opacity: 0.5,
            };
        }

        return {
            borderImageSource: gradient,
            borderImageSlice: 1,
            opacity: 1,
        };
    }, [hovering, dragState, selectedIndex, dragOffset, colors]);

    return (
        <div className={s.FilterWrapper} ref={wrapperRef}>
            <button
                className={`${s.FilterArrow} ${s.FilterArrowLeft}`}
                aria-label="Previous filter"
                onClick={() => applySelection(selectedIndex - 1)}
            >
                ‹
            </button>
            <div
                className={s.FilterSlider}
                ref={sliderRef}
                onPointerDown={(e) => {
                    e.preventDefault();
                    setDropdownOpen(false);
                    setDragState({ startX: e.clientX, moved: false });
                    (e.target as HTMLElement).setPointerCapture(e.pointerId);
                }}
                onPointerMove={(e) => {
                    if (!dragState || !sliderRef.current) {
                        return;
                    }
                    const delta = e.clientX - dragState.startX;
                    if (Math.abs(delta) > 4 && !dragState.moved) {
                        setDragState({ ...dragState, moved: true });
                    }
                    const width = sliderRef.current.clientWidth || 1;
                    setDragOffset(delta / width);
                }}
                onPointerUp={(e) => {
                    if (!dragState) {
                        return;
                    }

                    const delta = e.clientX - dragState.startX;
                    const width = sliderRef.current?.clientWidth || 1;
                    const normalized = delta / width;

                    if (!dragState.moved && Math.abs(delta) < 4) {
                        setDropdownOpen(o => !o);
                        setDragState(null);
                        setDragOffset(0);
                        return;
                    }

                    applySelection(Math.round(selectedIndex - normalized));
                }}
                onPointerLeave={() => {
                    if (dragState) {
                        applySelection(Math.round(selectedIndex - dragOffset));
                    }
                    setHovering(false);
                }}
                onPointerEnter={() => setHovering(true)}
            >
                <div className={s.FilterBorderOverlay} style={borderOverlayStyle} />
                <div
                    className={s.FilterSliderTrack}
                    style={{
                        transform: `translateX(${-(selectedIndex - dragOffset) * 100}%)`,
                        transition: dragState ? "none" : "transform 220ms ease",
                    }}
                >
                    {options.map((option, idx) => (
                        <div
                            key={option.label}
                            className={`${s.FilterSliderItem} ${idx === selectedIndex ? s.FilterSliderItemActive : ""}`}
                        >
                            <span>{option.label}</span>
                        </div>
                    ))}
                </div>
            </div>
            <button
                className={`${s.FilterArrow} ${s.FilterArrowRight}`}
                aria-label="Next filter"
                onClick={() => applySelection(selectedIndex + 1)}
            >
                ›
            </button>
            <div className={`${s.FilterDropdown} ${isDropdownOpen ? s.FilterDropdownOpen : ""}`}>
                {options.map((option, idx) => (
                    <div
                        key={option.label}
                        className={`${s.FilterDropdownItem} ${idx === selectedIndex ? s.FilterDropdownItemActive : ""}`}
                        onClick={() => {
                            onChange(option.value);
                            setDropdownOpen(false);
                        }}
                    >
                        {option.label}
                    </div>
                ))}
            </div>
        </div>
    );
};
