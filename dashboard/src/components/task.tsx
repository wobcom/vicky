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

type TaskProps = {
    task: ITask
}

const Task = (props: TaskProps) => {
    const { task } = props;
    const [localTask, setLocalTask] = useState<ITask>(task);
    const duration = localTask.finished_at && localTask.claimed_at ? Math.max(localTask.finished_at - localTask.claimed_at, 0) : null
    const api = useAPI();
    const [confirming, setConfirming] = useState(false);
    const [confirmError, setConfirmError] = useState<string | null>(null);

    useEffect(() => {
        setLocalTask(task);
    }, [task]);

    const needsValidation = localTask.status.state === "NEEDS_USER_VALIDATION" || localTask.status.state === "NEEDSUSERVALIDATION";

    const onConfirm = async () => {
        try {
            setConfirming(true);
            setConfirmError(null);
            await api.confirmTask(localTask.id);
            const fresh = await api.getTask(localTask.id);
            if (fresh) {
                setLocalTask(fresh);
            }
        } catch (e) {
            setConfirmError("Could not confirm task");
        } finally {
            setConfirming(false);
        }
    }

    return (
        <Panel shaded bordered className={s.Panel}>
            <VStack spacing={10} className={s.TitleStack}>
                <HStack justifyContent="space-between" alignItems="center" spacing={14} className={s.TitleRow}>
                    <HStack spacing={12} alignItems="center" className={s.TitleLeft}>
                        <h4 className={s.TitleText}>{localTask.display_name}</h4>
                        <TaskTag size="lg" task={localTask}/>
                    </HStack>
                    {needsValidation ? (
                        <Button size="sm" appearance="primary" onClick={onConfirm} loading={confirming}>
                            Confirm
                        </Button>
                    ) : null}
                </HStack>

                <VStack>
                    <HStack spacing={4}>
                        <CalendarIcon></CalendarIcon><Text muted>{dayjs.unix(localTask.created_at).toNow(true)} ago</Text>
                        {duration != null ? <Fragment>&mdash;</Fragment> : null}
                        {duration != null ? <Fragment><TimeIcon></TimeIcon><Text muted>{duration}s</Text></Fragment> : null}
                    </HStack>
                    {localTask.locks.length ? (
                        <HStack spacing={8} className={s.LockRow}>
                            {localTask.locks.map(lock => {
                                return (
                                    <Badge key={`${localTask.id}-${lock.name}`} color={lock.type === "WRITE" ? "red" : "green"} content={lock.type === "WRITE" ? "W" : "R"}>
                                        <Tag size="md" className={s.LockTag}>{lock.name}</Tag>
                                    </Badge>
                                )
                            })}
                        </HStack>
                    ) : null}

                </VStack>
            </VStack>
            {confirmError ? <Text color="red">{confirmError}</Text> : null}
            <Terminal key={localTask.id} taskId={localTask.id} />
        </Panel>
    )
}

export {
    Task
}
