import { Fragment, useState } from "react";
import { Link, useParams } from "react-router-dom"
import CalendarIcon from '@rsuite/icons/Calendar';
import TimeIcon from '@rsuite/icons/Time';

import { Col, Grid, HStack, List, Pagination, Panel, Text, VStack } from "rsuite"
import { TaskTag } from "./tag";
import { Task } from "./task";
import * as dayjs from "dayjs"

import * as s from "./tasks.module.css";
import { useTask, useTasks, useTasksCount } from "../hooks/useTasks";
import { FilterSlider } from "./filter-slider";

const FILTERS: { label: string; value: string | null; color: string }[] = [
    { label: "All", value: null, color: "#6b7280"},
    { label: "Validation", value: "NEEDS_USER_VALIDATION", color: "#f59e0b" },
    { label: "New", value: "NEW", color: "#22d3ee" },
    { label: "Running", value: "RUNNING", color: "#7c3aed" },
    { label: "Finished", value: "FINISHED::SUCCESS", color: "#22c55e" },
    { label: "Error", value: "FINISHED::ERROR", color: "#ef4444" },
];

const Tasks = () => {

    const { taskId } = useParams();

    const [filter, setFilter] = useState<string | null>(null);
    const [page, setPage] = useState<number>(1);

    const NUM_PER_PAGE = 10;

    const tasks = useTasks(filter, NUM_PER_PAGE, (page - 1) * NUM_PER_PAGE);
    const tasksCount = useTasksCount(filter);
    const task = useTask(taskId);

    return (
        <Grid fluid className={s.Grid}>
            <Col xs={task ? "8" : "24"}>
                <Panel shaded bordered className={s.TasksPanel}>
                    <HStack justifyContent="space-between" spacing={8}>
                        <h4>Tasks</h4>
                        <FilterSlider
                            options={FILTERS}
                            value={filter}
                            onChange={setFilter}
                        />
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
