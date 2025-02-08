import { Badge, HStack, Panel, Tag, Text, VStack } from "rsuite";
import { ITask } from "../services/api"
import { Terminal } from "./xterm";

import CalendarIcon from '@rsuite/icons/Calendar';
import TimeIcon from '@rsuite/icons/Time';
import * as dayjs from "dayjs";

import * as s from "./task.module.css";
import { Fragment, useMemo } from "react";
import { TaskTag } from "./tag";

type TaskProps = {
    task: ITask
}

const Task = (props: TaskProps) => {
    const { task } = props;
    const duration = task.finished_at && task.claimed_at ? Math.max(task.finished_at - task.claimed_at, 0) : null

    return (
        <Panel shaded bordered className={s.Panel}>
            <HStack justifyContent="space-between" spacing={20} className={s.TitleStack}>
                <VStack>
                    <h4>{task.display_name}</h4>
                    <HStack spacing={4}>
                        <CalendarIcon></CalendarIcon><Text muted>{dayjs.unix(task.created_at).toNow(true)} ago</Text>
                        {duration != null ? <Fragment>&mdash;</Fragment> : null}
                        {duration != null ? <Fragment><TimeIcon></TimeIcon><Text muted>{duration}s</Text></Fragment> : null}
                    </HStack>

                </VStack>
                <HStack spacing={30}>
                        {task.locks.map(lock => {
                            return (
                                <Badge color={lock.type === "WRITE" ? "red" : "green"} content={lock.type === "WRITE" ? "W" : "R"}>
                                    <Tag size="lg">{lock.name}</Tag>
                                </Badge>
                            )
                        })
                    }
                    <TaskTag size="lg" task={task}/>
                </HStack>
            </HStack>
            <Terminal key={task.id} taskId={task.id} />
        </Panel>
    )
}

export {
    Task
}