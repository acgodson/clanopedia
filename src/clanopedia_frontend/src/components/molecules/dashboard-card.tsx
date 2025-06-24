import * as React from "react"
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "../atoms/card"
import { Button } from "../atoms/button"
import { useToast } from "../../providers/toast"

interface DashboardCardProps {
  title: string
  description: string
  actionLabel?: string
  onAction?: () => void
  children?: React.ReactNode
}

export function DashboardCard({
  title,
  description,
  actionLabel,
  onAction,
  children
}: DashboardCardProps) {
  const { toast } = useToast()

  const handleAction = () => {
    if (onAction) {
      onAction()
    }
    toast({
      title: "Action taken",
      description: `You clicked the action on ${title}`,
    })
  }

  return (
    <Card className="w-full max-w-md">
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        <CardDescription>{description}</CardDescription>
      </CardHeader>
      <CardContent>
        {children}
      </CardContent>
      {actionLabel && (
        <CardFooter className="flex justify-between">
          <Button variant="ghost" onClick={() => toast({
            title: "Info",
            description: "This is a ghost button",
          })}>
            Info
          </Button>
          <Button onClick={handleAction}>
            {actionLabel}
          </Button>
        </CardFooter>
      )}
    </Card>
  )
} 