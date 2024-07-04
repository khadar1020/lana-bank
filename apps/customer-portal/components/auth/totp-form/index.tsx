"use client"
import { useState } from "react"

import { Button } from "@/components/primitive/button"
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@/components/primitive/card"
import { Input } from "@/components/primitive/input"
import { Alert, AlertDescription } from "@/components/primitive/alert"
import { submitTotpFow } from "@/lib/kratos/public/submit-totp"

const TotpForm = ({ flowId }: { flowId: string }) => {
  const [totpCode, setTotpCode] = useState("")
  const [error, setError] = useState<string | null>(null)

  const handleTotpSubmit = async (event: React.FormEvent) => {
    event.preventDefault()
    setError(null)
    if (!totpCode) {
      return
    }

    const submitTotpFowResponse = await submitTotpFow({
      flowId,
      totpCode,
    })

    if (submitTotpFowResponse instanceof Error) {
      setError(submitTotpFowResponse.message)
      return
    }

    window.location.href = "/"
  }

  return (
    <Card variant="transparent" className="md:w-2/5">
      <CardHeader className="pt-4">
        <CardTitle>Authenticator Code</CardTitle>
        <CardDescription className="text-textColor-secondary">
          Please enter your authenticator code to continue.
        </CardDescription>
      </CardHeader>
      <form onSubmit={handleTotpSubmit}>
        <CardContent>
          <Input
            type="text"
            value={totpCode}
            onChange={(e) => setTotpCode(e.target.value)}
            placeholder="Please enter code"
          />
        </CardContent>
        <CardFooter className="flex flex-col gap-2">
          <Button type="submit" className="rounded-full px-6 w-full">
            Next
          </Button>
          {error && (
            <Alert variant="destructive" className="mt-1 p-3">
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          )}
        </CardFooter>
      </form>
    </Card>
  )
}

export { TotpForm }
