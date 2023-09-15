import { useEffect, useMemo, useState } from "react";
import { Link, useParams } from "react-router-dom"
import { Col, Grid, List, Panel, Row, Stack, Tag } from "rsuite"
import { ITask, useAPI } from "../services/api";
import { Task } from "./task";
import { Terminal } from "./xterm"

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
        api.getTasks().then((tasks) => setTasks(tasks))
    }, [])

    return (
        <Grid fluid>
            <Col xs="8">
                <Panel shaded bordered>
                    <h4>Tasks</h4>
                    <List>
                        {
                            tasks && tasks.map((t) => {

                                return (
                                    <List.Item>
                                        <Link to={`/tasks/${t.id}`}>{t.display_name}</Link>
                                    </List.Item>
                                )
                            })
                        }
                    </List>
                </Panel>

            </Col>
            <Col xs="16">
                {
                    task ? (
                            <Row>
                                <Task task={task}/>
                            </Row>
                    ) : null
                }

            </Col>
        </Grid>
    )
}

export {
    Tasks
}