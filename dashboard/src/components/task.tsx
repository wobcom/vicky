import { Badge, Panel, Stack, Tag } from "rsuite";
import { ITask } from "../services/api"
import { Terminal } from "./xterm";

import * as s from "./task.module.css";
import { useMemo } from "react";

type TaskProps = {
    task: ITask
}

const Task = (props: TaskProps) => {
    const { task } = props;

    const [tagContent, tagColor] = useMemo(() => {
        const tagContent = task.status.result ?? task.status.state

        let tagColor = null
        let tagDisplay = null
        switch (tagContent) {
            case "ERROR": {
                tagColor = "red";
                tagDisplay = "Error";
                break;
            }
            case "SUCCESS": {
                tagColor = "green";
                tagDisplay = "Success";
                break;
            }
            case "RUNNING": {
                tagColor = "violet";
                tagDisplay = "Running";
                break;
            }
            case "NEW": {
                tagColor = "cyan";
                tagDisplay = "New";
                break;
            }
            default: {
                tagColor = "";
                tagDisplay = "-"
            }
        }

        return [tagDisplay, tagColor]

    }, [task])

    console.log(task)

    return (
        <Panel shaded bordered>
            <Stack justifyContent="space-between" spacing={20} className={s.titleStack}>
                <h4>{task.display_name}</h4>

                <Stack spacing={30}>
                    {
                        task.locks.map(lock => {
                            return (
                                <Badge color={lock.type === "WRITE" ? "red" : "green"} content={lock.type === "WRITE" ? "W" : "R"}>
                                    <Tag size="lg">{lock.object}</Tag>
                                </Badge>
                            )
                        })
                    }
                    <Tag color={tagColor} size="lg">{tagContent}</Tag>
                </Stack>
            </Stack>
            <Terminal key={task.id} taskId={task.id} />
        </Panel>
    )
}

export {
    Task
}