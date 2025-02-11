// this file is autogenerated by codegen
/* eslint-disable */
import { gql } from '@apollo/client';
import * as Apollo from '@apollo/client';
export type Maybe<T> = T | null;
export type InputMaybe<T> = Maybe<T>;
export type Exact<T extends { [key: string]: unknown }> = { [K in keyof T]: T[K] };
export type MakeOptional<T, K extends keyof T> = Omit<T, K> & { [SubKey in K]?: Maybe<T[SubKey]> };
export type MakeMaybe<T, K extends keyof T> = Omit<T, K> & { [SubKey in K]: Maybe<T[SubKey]> };
export type MakeEmpty<T extends { [key: string]: unknown }, K extends keyof T> = { [_ in K]?: never };
export type Incremental<T> = T | { [P in keyof T]?: P extends ' $fragmentName' | '__typename' ? T[P] : never };
const defaultOptions = {} as const;
/** All built-in and custom scalars, mapped to their actual values */
export type Scalars = {
  ID: { input: string; output: string; }
  String: { input: string; output: string; }
  Boolean: { input: boolean; output: boolean; }
  Int: { input: number; output: number; }
  Float: { input: number; output: number; }
  Timestamp: { input: any; output: any; }
  UUID: { input: any; output: any; }
  UsdCents: { input: any; output: any; }
};

export enum AccountStatus {
  Active = 'ACTIVE',
  Inactive = 'INACTIVE'
}

export type Customer = {
  __typename?: 'Customer';
  createdAt: Scalars['Timestamp']['output'];
  customerId: Scalars['UUID']['output'];
  depositAccount: DepositAccount;
  email: Scalars['String']['output'];
  id: Scalars['ID']['output'];
  level: KycLevel;
  status: AccountStatus;
  telegramId: Scalars['String']['output'];
};

export type Deposit = {
  __typename?: 'Deposit';
  accountId: Scalars['UUID']['output'];
  amount: Scalars['UsdCents']['output'];
  createdAt: Scalars['Timestamp']['output'];
  depositId: Scalars['UUID']['output'];
  id: Scalars['ID']['output'];
  reference: Scalars['String']['output'];
};

export type DepositAccount = {
  __typename?: 'DepositAccount';
  balance: DepositAccountBalance;
  createdAt: Scalars['Timestamp']['output'];
  customerId: Scalars['UUID']['output'];
  depositAccountId: Scalars['UUID']['output'];
  deposits: Array<Deposit>;
  id: Scalars['ID']['output'];
  withdrawals: Array<Withdrawal>;
};

export type DepositAccountBalance = {
  __typename?: 'DepositAccountBalance';
  pending: Scalars['UsdCents']['output'];
  settled: Scalars['UsdCents']['output'];
};

export enum KycLevel {
  Advanced = 'ADVANCED',
  Basic = 'BASIC',
  NotKyced = 'NOT_KYCED'
}

export type Query = {
  __typename?: 'Query';
  me: Subject;
};

export type Subject = {
  __typename?: 'Subject';
  customer: Customer;
};

export type Withdrawal = {
  __typename?: 'Withdrawal';
  accountId: Scalars['UUID']['output'];
  amount: Scalars['UsdCents']['output'];
  createdAt: Scalars['Timestamp']['output'];
  id: Scalars['ID']['output'];
  reference: Scalars['String']['output'];
  status: WithdrawalStatus;
  withdrawalId: Scalars['UUID']['output'];
};

export enum WithdrawalStatus {
  Cancelled = 'CANCELLED',
  Confirmed = 'CONFIRMED',
  Denied = 'DENIED',
  PendingApproval = 'PENDING_APPROVAL',
  PendingConfirmation = 'PENDING_CONFIRMATION'
}

export type MeQueryVariables = Exact<{ [key: string]: never; }>;


export type MeQuery = { __typename?: 'Query', me: { __typename?: 'Subject', customer: { __typename?: 'Customer', id: string, customerId: any, status: AccountStatus, level: KycLevel, createdAt: any, email: string, telegramId: string, depositAccount: { __typename?: 'DepositAccount', id: string, depositAccountId: any, customerId: any, createdAt: any, balance: { __typename?: 'DepositAccountBalance', settled: any, pending: any }, deposits: Array<{ __typename?: 'Deposit', id: string, depositId: any, accountId: any, amount: any, createdAt: any, reference: string }>, withdrawals: Array<{ __typename?: 'Withdrawal', id: string, withdrawalId: any, accountId: any, amount: any, createdAt: any, reference: string, status: WithdrawalStatus }> } } } };


export const MeDocument = gql`
    query me {
  me {
    customer {
      id
      customerId
      status
      level
      createdAt
      email
      telegramId
      depositAccount {
        id
        depositAccountId
        customerId
        createdAt
        balance {
          settled
          pending
        }
        deposits {
          id
          depositId
          accountId
          amount
          createdAt
          reference
        }
        withdrawals {
          id
          withdrawalId
          accountId
          amount
          createdAt
          reference
          status
        }
      }
    }
  }
}
    `;

/**
 * __useMeQuery__
 *
 * To run a query within a React component, call `useMeQuery` and pass it any options that fit your needs.
 * When your component renders, `useMeQuery` returns an object from Apollo Client that contains loading, error, and data properties
 * you can use to render your UI.
 *
 * @param baseOptions options that will be passed into the query, supported options are listed on: https://www.apollographql.com/docs/react/api/react-hooks/#options;
 *
 * @example
 * const { data, loading, error } = useMeQuery({
 *   variables: {
 *   },
 * });
 */
export function useMeQuery(baseOptions?: Apollo.QueryHookOptions<MeQuery, MeQueryVariables>) {
        const options = {...defaultOptions, ...baseOptions}
        return Apollo.useQuery<MeQuery, MeQueryVariables>(MeDocument, options);
      }
export function useMeLazyQuery(baseOptions?: Apollo.LazyQueryHookOptions<MeQuery, MeQueryVariables>) {
          const options = {...defaultOptions, ...baseOptions}
          return Apollo.useLazyQuery<MeQuery, MeQueryVariables>(MeDocument, options);
        }
export type MeQueryHookResult = ReturnType<typeof useMeQuery>;
export type MeLazyQueryHookResult = ReturnType<typeof useMeLazyQuery>;
export type MeQueryResult = Apollo.QueryResult<MeQuery, MeQueryVariables>;