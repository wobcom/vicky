import { useEffect, useMemo, useState } from "react";
import { Link, useParams } from "react-router-dom"
import { Col, Grid, List, Panel, Row, Stack, Tag } from "rsuite"
import { ITask, useAPI } from "../services/api";
import { TaskTag } from "./tag";
import { Task } from "./task";
import { Terminal } from "./xterm"

import * as s from "./tasks.module.css";
import { useTask, useTasks } from "../hooks/useTasks";

const Tasks = () => {

    const { taskId } = useParams();
    const api = useAPI();

    const tasks = useTasks();
    const task = useTask(taskId)

    return (
        <Grid fluid className={s.Grid}>
            <Col xs={task ? "8" : "24"}>
                <Panel shaded bordered class>
                    <h4>Tasks</h4>
                    <List bordered className={s.List}>
                        {
                            tasks?.map((t) => {
                                const isSelected = t.id == task?.id;

                                return (
                                    <Link key={t.id} to={`/tasks/${t.id}`}>
                                    <List.Item key={t.id} className={isSelected ? s.ListItemSelected : ""}>
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
            {
	        task ? (
                    <Col xs="16" className={s.GridElement}>
                        <Task task={task}/>
                    </Col>
                ) : null
	    }
        </Grid>
    )
}

export {
    Tasks
}
