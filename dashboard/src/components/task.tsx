import { Badge, Button, HStack, Panel, Tag, Text, VStack } from "rsuite";
import { ITask } from "../services/api"
import { Terminal } from "./xterm";

import CalendarIcon from '@rsuite/icons/Calendar';
import TimeIcon from '@rsuite/icons/Time';
import * as dayjs from "dayjs";

import * as s from "./task.module.css";
import { Fragment, useEffect, useMemo, useState } from "react";
import { TaskTag } from "./tag";
import { useAPI } from "../services/api";
import {FILTERS} from "./tasks";

type TaskProps = {
    task: ITask
}

const Task = (props: TaskProps) => {
    const { task } = props;
    const duration = task.finished_at && task.claimed_at ? Math.max(task.finished_at - task.claimed_at, 0) : null
    const api = useAPI();
    const [confirming, setConfirming] = useState(false);
    const [confirmError, setConfirmError] = useState<string | null>(null);

    const needsValidation = task.status.state === "NEEDS_USER_VALIDATION" || task.status.state === "NEEDSUSERVALIDATION";

    const onConfirm = async () => {
        try {
            setConfirming(true);
            setConfirmError(null);
            await api.confirmTask(task.id);
        } catch (e) {
            setConfirmError("Could not confirm task");
        } finally {
            setConfirming(false);
        }
    }

    return (
        <Panel shaded borderLeft={"1px solid var(--rs-gray-700)"} className={s.Panel}>
            <HStack alignItems={"flex-start"} justifyContent="space-between" className={s.TaskHeader}>
                <VStack spacing={10} className={s.TitleStack}>
                    <HStack justifyContent="space-between" alignItems="center" spacing={14} className={s.TitleRow}>
                        <HStack spacing={12} alignItems="center" className={s.TitleLeft}>
                            <h4 className={s.TitleText}>{task.display_name}</h4>
                            <TaskTag size="lg" task={task} options={FILTERS}/>
                        </HStack>
                    </HStack>

                    <VStack>
                        <HStack spacing={4}>
                            <CalendarIcon></CalendarIcon><Text muted>{dayjs.unix(task.created_at).toNow(true)} ago</Text>
                            {duration != null ? <Fragment>&mdash;</Fragment> : null}
                            {duration != null ? <Fragment><TimeIcon></TimeIcon><Text muted>{duration}s</Text></Fragment> : null}
                        </HStack>
                        {task.locks.length ? (
                            <HStack spacing={8} className={s.LockRow}>
                                {task.locks.map(lock => {
                                    return (
                                        <Badge
                                            key={`${task.id}-${lock.name}`}
                                            color={lock.type === "WRITE" ? "red" : "green"}
                                            content={lock.type === "WRITE" ? "W" : "R"}
                                            offset={[0, 0]}
                                        >
                                            <Tag size="md" className={s.LockTag} color={lock.poisoned ? "red" : null}>{lock.name}</Tag>
                                        </Badge>
                                    )
                                })}
                            </HStack>
                        ) : null}

                    </VStack>
                </VStack>
                <VStack spacing={10} alignItems="flex-end" justify="space-between">
                    {
                        task.group != null ?
                            <Text muted>Group: {task.group}</Text>
                            : null
                    }
                    {needsValidation ? (
                        <Button size="sm" appearance="primary" onClick={onConfirm} loading={confirming}>
                            Confirm
                        </Button>
                    ) : null}
                </VStack>
            </HStack>
            {confirmError ? <Text color="red">{confirmError}</Text> : null}
            <Terminal key={task.id} taskId={task.id} />
        </Panel>
    )
}

export {
    Task
}
