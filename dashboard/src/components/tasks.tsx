import { Fragment, useEffect, useState } from "react";
import { Link, useParams } from "react-router-dom"
import CalendarIcon from '@rsuite/icons/Calendar';
import TimeIcon from '@rsuite/icons/Time';

import {Col, Grid, Heading, HStack, List, Pagination, Panel, Row, Text, VStack} from "rsuite"
import { TaskTag } from "./tag";
import { Task } from "./task";
import { FilterSlider } from "./filter-slider";
import * as dayjs from "dayjs"

import * as s from "./tasks.module.css";
import { useTask, useTasks, useTasksCount } from "../hooks/useTasks";
import { useTaskGroups } from "../hooks/useTaskGroups";
import { GroupFilter } from "./group-filter";


export type FilterValue = string | null;
export type FilterOption = { label: string; value: FilterValue; color: string };

export const FILTERS: FilterOption[] = [
    { label: "All", value: null, color: "#6b7280"},
    { label: "Validation", value: "NEEDS_USER_VALIDATION", color: "#f59e0b" },
    { label: "New", value: "NEW", color: "#22d3ee" },
    { label: "Running", value: "RUNNING", color: "#7c3aed" },
    { label: "Success", value: "FINISHED::SUCCESS", color: "#22c55e" },
    { label: "Timeout", value: "FINISHED::TIMEOUT", color: "#ff6200" },
    { label: "Cancelled", value: "FINISHED::CANCEL", color: "#5f656bff" },
    { label: "Error", value: "FINISHED::ERROR", color: "#ef4444" },
];

const Tasks = () => {

    const { taskId } = useParams();

    const [status, setStatus] = useState<string | null>(null);
    const [group, setGroup] = useState<string | null>(null);
    const [page, setPage] = useState<number>(1);

    const groups = useTaskGroups();

    const NUM_PER_PAGE = 10;

    const tasks = useTasks(status, group, NUM_PER_PAGE, (page - 1) * NUM_PER_PAGE);
    const tasksCount = useTasksCount(status, group);
    const task = useTask(taskId);

    useEffect(() => {
        setPage(1);
    }, [status, group]);

    return (
        <Grid fluid className={s.Grid}>
            <Row className={s.Row}>
                <Col span={{ xs: task ? 8 : 24 }} height="100%" className={s.TasksColumn}>
                    <Panel shaded className={s.TasksPanel}>
                        <HStack justifyContent="space-between" spacing={8} alignItems="center" className={s.HeaderRow}>
                            <Heading>Tasks</Heading>
                            <HStack spacing={8} alignItems="center" className={s.Filters}>
                                <FilterSlider
                                    options={FILTERS}
                                    value={status}
                                    onChange={setStatus}
                                />
                                <GroupFilter
                                    groups={groups}
                                    value={group}
                                    onChange={setGroup}
                                />
                            </HStack>
                        </HStack>


                        <List bordered className={s.List}>
                            {
                                tasks?.map((t) => {
                                    const isSelected = t.id == task?.id;
                                    const duration = t.finished_at && t.claimed_at ? Math.max(t.finished_at - t.claimed_at, 0) : null

                                    return (
                                        <Link key={t.id} to={`/tasks/${t.id}`} style={{textDecoration: "none"}}>
                                            <List.Item key={t.id} className={isSelected ? s.ListItemSelected : ""}>
                                                <HStack justifyContent="space-between" spacing={8} alignItems="center" className={s.ListRow}>
                                                    <VStack spacing={2}>
                                                        <span>{t.display_name}</span>
                                                        <HStack spacing={4}>
                                                            <CalendarIcon></CalendarIcon><Text muted>{dayjs.unix(t.created_at).toNow(true)} ago</Text>
                                                            {duration != null ? <Fragment>&mdash;</Fragment> : null}
                                                            {duration != null ? <Fragment><TimeIcon></TimeIcon><Text muted>{duration}s</Text></Fragment> : null}
                                                        </HStack>

                                                    </VStack>
                                                    <TaskTag size="sm" task={t} options={FILTERS}></TaskTag>
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
                                <Pagination next prev maxButtons={5} ellipsis boundaryLinks total={tasksCount} limit={NUM_PER_PAGE} activePage={page} onChangePage={(p: number) => setPage(p)} />
                            )
                            : null
                        }
                        </div>
                    </Panel>

                </Col>
                {
                    task ? (
                        <Col span={{ xs: 16 }} className={s.GridElement}>
                            <Task task={task} />
                        </Col>
                    ) : null
                }
            </Row>
        </Grid>
    )
}

export {
    Tasks
}
