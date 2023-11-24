import { useEffect, useMemo, useState } from "react";
import { Link, useParams } from "react-router-dom"
import { Col, Grid, List, Panel, Row, Stack, Tag } from "rsuite"
import { ITask, useAPI } from "../services/api";
import { TaskTag } from "./tag";
import { Task } from "./task";
import { Terminal } from "./xterm"

import * as s from "./tasks.module.css";

const Tasks = () => {

    const { taskId } = useParams();
    const api = useAPI();

    const [tasks, setTasks] = useState<ITask[] | null>(null);
    const task = useMemo(() => {
        if (!tasks || !taskId) {
            return null;
        }
        return tasks.find(t => t.id == taskId)
    }, [tasks, taskId])

    useEffect(() => {
        api.getTasks().then((tasks) => setTasks(tasks));

        // TODO: Implement websocket or long polling
        const interval = setInterval(() => {
            api.getTasks().then((tasks) => setTasks(tasks));
        }, 1000);

        return () => clearInterval(interval);
    }, [])

    return (
        <Grid fluid className={s.Grid}>
            <Col xs="8">
                <Panel shaded bordered class>
                    <h4>Tasks</h4>
                    <List bordered className={s.List}>
                        {
                            tasks && tasks.map((t) => {
                                const isSelected = t.id == task?.id;

                                return (
                                    <Link to={`/tasks/${t.id}`}>
                                    <List.Item className={isSelected ? s.ListItemSelected : ""}>
                                        <Stack direction="row" justifyContent="space-between" spacing={8}>
                                            <span>{t.display_name}</span>
                                            <TaskTag size="sm" task={t}></TaskTag>
                                        </Stack>
                                    </List.Item>
                                    </Link>
                                )
                            })
                        }
                    </List>
                </Panel>

            </Col>
            <Col xs="16" className={s.GridElement}>
                {
                    task ? (
                        <Task task={task}/>
                    ) : null
                }

            </Col>
        </Grid>
    )
}

export {
    Tasks
}