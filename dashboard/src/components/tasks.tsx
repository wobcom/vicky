import { Fragment, useEffect, useMemo, useRef, useState, CSSProperties } from "react";
import { Link, useParams } from "react-router-dom"
import CalendarIcon from '@rsuite/icons/Calendar';
import TimeIcon from '@rsuite/icons/Time';

import { Col, Grid, HStack, List, Pagination, Panel, Text, VStack } from "rsuite"
import { TaskTag } from "./tag";
import { Task } from "./task";
import * as dayjs from "dayjs"

import * as s from "./tasks.module.css";
import { useTask, useTasks, useTasksCount } from "../hooks/useTasks";

const FILTERS: { label: string; value: string | null }[] = [
    { label: "All", value: null },
    { label: "Validation", value: "NEEDS_USER_VALIDATION" },
    { label: "New", value: "NEW" },
    { label: "Running", value: "RUNNING" },
    { label: "Finished", value: "FINISHED::SUCCESS" },
    { label: "Error", value: "FINISHED::ERROR" },
];
const FILTER_COLORS = [
    "#6b7280", // All
    "#f59e0b", // Validation
    "#22d3ee", // New
    "#7c3aed", // Running
    "#22c55e", // Finished
    "#ef4444", // Error
];

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

const Tasks = () => {

    const { taskId } = useParams();

    const [filter, setFilter] = useState<string | null>(null);
    const [isDropdownOpen, setDropdownOpen] = useState(false);
    const [dragOffset, setDragOffset] = useState(0);
    const [dragState, setDragState] = useState<{ startX: number; moved: boolean } | null>(null);
    const [page, setPage] = useState<number>(1);
    const sliderRef = useRef<HTMLDivElement | null>(null);
    const filterRef = useRef<HTMLDivElement | null>(null);
    const [hovering, setHovering] = useState(false);

    const selectedIndex = useMemo(() => {
        const idx = FILTERS.findIndex(f => f.value === filter);
        return idx >= 0 ? idx : 0;
    }, [filter]);

    const NUM_PER_PAGE = 10;

    const tasks = useTasks(filter, NUM_PER_PAGE, (page - 1) * NUM_PER_PAGE);
    const tasksCount = useTasksCount(filter);
    const task = useTask(taskId);

    const borderOverlayStyle = useMemo<CSSProperties>(() => {
        // little bit cursed but needed so it doesn't overflow and crash
        const pos = Math.max(0, Math.min(1, (selectedIndex - dragOffset) / (FILTERS.length - 1 || 1)));
        const nextIdx = Math.max(0, Math.min(FILTER_COLORS.length - 1, Math.ceil(selectedIndex - dragOffset)));
        const prevIdx = Math.max(0, Math.min(FILTER_COLORS.length - 1, Math.floor(selectedIndex - dragOffset)));
        const currentColor = FILTER_COLORS[prevIdx];
        const nextColor = FILTER_COLORS[nextIdx];
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
            borderImage: `${gradient} 1`,
            borderImageSlice: 1,
            opacity: 1,
            backgroundPosition: `${pos * 100}% 50%`,
        };
    }, [hovering, dragState, selectedIndex, dragOffset]);

    useEffect(() => {
        if (!isDropdownOpen) {
            return;
        }
        const handler = (e: MouseEvent) => {
            if (filterRef.current && !filterRef.current.contains(e.target as Node)) {
                setDropdownOpen(false);
            }
        };
        document.addEventListener("click", handler);
        return () => document.removeEventListener("click", handler);
    }, [isDropdownOpen]);

    return (
        <Grid fluid className={s.Grid}>
            <Col xs={task ? "8" : "24"}>
                <Panel shaded bordered className={s.TasksPanel}>
                    <HStack justifyContent="space-between" spacing={8}>

                        <h4>Tasks</h4>
                        <div className={s.FilterWrapper} ref={filterRef}>
                            <button
                                className={`${s.FilterArrow} ${s.FilterArrowLeft}`}
                                aria-label="Previous filter"
                                onClick={() => {
                                    const target = Math.max(0, selectedIndex - 1);
                                    setFilter(FILTERS[target].value);
                                }}
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

                                    const target = Math.round(selectedIndex - normalized);
                                    const clamped = Math.max(0, Math.min(FILTERS.length - 1, target));
                                    setFilter(FILTERS[clamped].value);
                                    setDragState(null);
                                    setDragOffset(0);
                                }}
                                onPointerLeave={() => {
                                    if (dragState) {
                                        const target = Math.round(selectedIndex - dragOffset);
                                        const clamped = Math.max(0, Math.min(FILTERS.length - 1, target));
                                        setFilter(FILTERS[clamped].value);
                                        setDragState(null);
                                        setDragOffset(0);
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
                                    {FILTERS.map((f, idx) => (
                                        <div
                                            key={f.label}
                                            className={`${s.FilterSliderItem} ${idx === selectedIndex ? s.FilterSliderItemActive : ""}`}
                                        >
                                            <span>{f.label}</span>
                                        </div>
                                    ))}
                                </div>
                            </div>
                            <button
                                className={`${s.FilterArrow} ${s.FilterArrowRight}`}
                                aria-label="Next filter"
                                onClick={() => {
                                    const target = Math.min(FILTERS.length - 1, selectedIndex + 1);
                                    setFilter(FILTERS[target].value);
                                }}
                            >
                                ›
                            </button>
                            <div className={`${s.FilterDropdown} ${isDropdownOpen ? s.FilterDropdownOpen : ""}`}>
                                {FILTERS.map((opt, idx) => (
                                    <div
                                        key={opt.label}
                                        className={`${s.FilterDropdownItem} ${idx === selectedIndex ? s.FilterDropdownItemActive : ""}`}
                                        onClick={() => {
                                            setFilter(opt.value);
                                            setDropdownOpen(false);
                                        }}
                                    >
                                        {opt.label}
                                    </div>
                                ))}
                            </div>
                        </div>
                    </HStack>


                    <List bordered className={s.List}>
                        {
                            tasks?.map((t) => {
                                const isSelected = t.id == task?.id;
                                const duration = t.finished_at && t.claimed_at ? Math.max(t.finished_at - t.claimed_at, 0) : null

                                return (
                                    <Link key={t.id} to={`/tasks/${t.id}`} style={{textDecoration: "none"}}>
                                        <List.Item key={t.id} className={isSelected ? s.ListItemSelected : ""}>
                                            <HStack justifyContent="space-between" spacing={8}>
                                                <VStack spacing={2}>
                                                    <span>{t.display_name}</span>
                                                    <HStack spacing={4}>
                                                        <CalendarIcon></CalendarIcon><Text muted>{dayjs.unix(t.created_at).toNow(true)} ago</Text>
                                                        {duration != null ? <Fragment>&mdash;</Fragment> : null}
                                                        {duration != null ? <Fragment><TimeIcon></TimeIcon><Text muted>{duration}s</Text></Fragment> : null}
                                                    </HStack>

                                                </VStack>
                                                <TaskTag size="sm" task={t}></TaskTag>
                                            </HStack>
                                        </List.Item>
                                    </Link>
                                )
                            })
                        }
                    </List>
                    <div className={s.Pagination}>
                    {
                        tasksCount ?
                        (
                            <Pagination bordered next prev maxButtons={10} total={tasksCount} limit={NUM_PER_PAGE} activePage={page} onChangePage={(p: number) => setPage(p)} />
                        )
                        : null
                    }
                    </div>
                </Panel>

            </Col>
            {
                task ? (
                    <Col xs="16" className={s.GridElement}>
                        <Task task={task} />
                    </Col>
                ) : null
            }
        </Grid>
    )
}

export {
    Tasks
}
