import { Badge, Panel, Stack, Tag } from "rsuite";
import { ITask } from "../services/api"
import { Terminal } from "./xterm";

import * as s from "./task.module.css";
import { useMemo } from "react";
import { TaskTag } from "./tag";

type TaskProps = {
    task: ITask
}

const Task = (props: TaskProps) => {
    const { task } = props;

    return (
        <Panel shaded bordered className={s.Panel}>
            <Stack justifyContent="space-between" spacing={20} className={s.TitleStack}>
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
                    <TaskTag size="lg" task={task}/>
                </Stack>
            </Stack>
            <Terminal key={task.id} taskId={task.id} />
        </Panel>
    )
}

export {
    Task
}