"use client"

import { gql } from "@apollo/client"

import Balance from "@/components/balance/balance"
import { DetailItem } from "@/components/details"
import { LoanApproveDialog } from "@/components/loan/approve-loan"
import { LoanBadge } from "@/components/loan/loan-badge"
import { LoanPartialPaymentDialog } from "@/components/loan/loan-partial-payment"
import { Button } from "@/components/primitive/button"
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/primitive/card"
import { Separator } from "@/components/primitive/separator"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/primitive/table"
import {
  LoanStatus,
  useGetLoanDetailsQuery,
  TransactionType,
} from "@/lib/graphql/generated"
import { formatInterval, formatPeriod } from "@/lib/term/utils"
import { formatDate } from "@/lib/utils"

gql`
  query GetLoanDetails($id: UUID!) {
    loan(id: $id) {
      id
      loanId
      createdAt
      status
      customer {
        customerId
      }
      balance {
        collateral {
          btcBalance
        }
        outstanding {
          usdBalance
        }
        interestIncurred {
          usdBalance
        }
      }
      transactions {
        amount
        transactionType
        recordedAt
      }
      loanTerms {
        annualRate
        interval
        liquidationCvl
        marginCallCvl
        initialCvl
        duration {
          period
          units
        }
      }
    }
  }
`

type LoanDetailsProps = { loanId: string }

const LoanDetails: React.FC<LoanDetailsProps> = ({ loanId }) => {
  const {
    data: loanDetails,
    loading,
    error,
    refetch,
  } = useGetLoanDetailsQuery({ variables: { id: loanId } })

  return (
    <>
      <Card>
        {loading ? (
          <CardContent className="pt-6">Loading...</CardContent>
        ) : error ? (
          <CardContent className="pt-6 text-destructive">{error.message}</CardContent>
        ) : loanDetails?.loan ? (
          <>
            <CardHeader className="flex flex-row justify-between items-center">
              <div>
                <h2 className="font-semibold leading-none tracking-tight">Loan</h2>
                <p className="text-textColor-secondary text-sm mt-2">
                  {loanDetails.loan.loanId}
                </p>
              </div>
              <div className="flex flex-col gap-2">
                <LoanBadge status={loanDetails.loan.status} className="p-1 px-4" />
              </div>
            </CardHeader>
            <Separator className="mb-6" />
            <CardContent>
              <div className="grid grid-cols-2 gap-6">
                <div className="grid auto-rows-min ">
                  <DetailItem
                    label="Customer ID"
                    value={loanDetails.loan.customer.customerId}
                  />
                  <DetailItem
                    label="Created At"
                    value={formatDate(loanDetails.loan.createdAt)}
                  />
                  <DetailItem
                    label="Collateral balance (BTC)"
                    valueComponent={
                      <Balance
                        amount={loanDetails.loan.balance.collateral.btcBalance}
                        currency="btc"
                      />
                    }
                  />
                  <DetailItem
                    label="Outstanding balance (USD)"
                    valueComponent={
                      <Balance
                        amount={loanDetails.loan.balance.outstanding.usdBalance}
                        currency="usd"
                      />
                    }
                  />
                  <DetailItem
                    label="Interest Incurred (USD)"
                    valueComponent={
                      <Balance
                        amount={loanDetails.loan.balance.interestIncurred.usdBalance}
                        currency="usd"
                      />
                    }
                  />
                  <DetailItem
                    label="Initial CVL"
                    value={`${loanDetails.loan.loanTerms.initialCvl}%`}
                  />
                </div>
                <div className="grid auto-rows-min">
                  <DetailItem
                    label="Duration"
                    value={`${loanDetails.loan.loanTerms.duration.units} ${formatPeriod(loanDetails.loan.loanTerms.duration.period)}`}
                  />
                  <DetailItem
                    label="Interval"
                    value={formatInterval(loanDetails.loan.loanTerms.interval)}
                  />
                  <DetailItem
                    label="Annual Rate"
                    value={`${loanDetails.loan.loanTerms.annualRate}%`}
                  />
                  <DetailItem
                    label="Liquidation CVL"
                    value={`${loanDetails.loan.loanTerms.liquidationCvl}%`}
                  />
                  <DetailItem
                    label="Margin Call CVL"
                    value={`${loanDetails.loan.loanTerms.marginCallCvl}%`}
                  />
                </div>
              </div>
            </CardContent>
            {loanDetails.loan.status !== LoanStatus.Closed && (
              <Separator className="mb-6" />
            )}
            <div className="flex flex-row gap-2 p-6 pt-0 mt-0">
              {loanDetails.loan.status === LoanStatus.Active && (
                <LoanPartialPaymentDialog
                  refetch={refetch}
                  loanId={loanDetails.loan.loanId}
                >
                  <Button>Make Payment</Button>
                </LoanPartialPaymentDialog>
              )}
              {loanDetails.loan.status === LoanStatus.New && (
                <LoanApproveDialog refetch={refetch} loanId={loanDetails.loan.loanId}>
                  <Button>Approve Loan</Button>
                </LoanApproveDialog>
              )}
            </div>
          </>
        ) : (
          loanId &&
          !loanDetails?.loan && (
            <CardContent className="pt-6">No loan found with this ID</CardContent>
          )
        )}
      </Card>
      <Card className="mt-4">
        <CardHeader>
          <CardTitle>Loan Transactions</CardTitle>
        </CardHeader>
        <CardContent>
          {loading ? (
            <p>Loading...</p>
          ) : error ? (
            <p className="text-destructive">{error.message}</p>
          ) : loanDetails?.loan?.transactions.length ? (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Transaction Type</TableHead>
                  <TableHead>Recorded At</TableHead>
                  <TableHead className="text-right">Amount (USD)</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {loanDetails.loan.transactions.map((transaction, index) => (
                  <TableRow key={index}>
                    <TableCell>
                      {formatLoanTransactionType(transaction.transactionType)}
                    </TableCell>
                    <TableCell>{formatDate(transaction.recordedAt)}</TableCell>
                    <TableCell className="text-right">
                      <Balance amount={transaction.amount} currency="usd" />
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          ) : (
            <CardDescription>No transactions found</CardDescription>
          )}
        </CardContent>
      </Card>
    </>
  )
}

const formatLoanTransactionType = (transactionType: TransactionType) => {
  return transactionType
    .toLowerCase()
    .split("_")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ")
}

export default LoanDetails